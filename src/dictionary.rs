use crate::embeddings::{get_embeddings, is_valid_word};
use bincode::{Decode, Encode};
use merriam_webster_http::MerriamWebsterClient;
use moka::future::Cache;
use rand::prelude::IteratorRandom;
use rand::rng;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::OnceLock;
use teloxide::payloads::{
    EditMessageReplyMarkupSetters, EditMessageTextSetters, SendMessageSetters,
};
use teloxide::prelude::{Requester, ResponseResult};
use teloxide::types::ParseMode::MarkdownV2;
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, MessageId};
use teloxide::Bot;

/// Custom error type for dictionary operations
#[derive(Debug)]
pub enum DictionaryError {
    NotFound(String),
    ApiError(String),
    CacheError(String),
    IoError(std::io::Error),
}

impl fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DictionaryError::NotFound(word) => write!(f, "Word '{}' not found", word),
            DictionaryError::ApiError(msg) => write!(f, "API error: {}", msg),
            DictionaryError::CacheError(msg) => write!(f, "Cache error: {}", msg),
            DictionaryError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl From<std::io::Error> for DictionaryError {
    fn from(error: std::io::Error) -> Self {
        DictionaryError::IoError(error)
    }
}

/// Word information including definitions and stems
#[derive(Encode, Decode, Clone, Debug)]
pub struct WordInfo {
    pub word: String,
    pub stems: Vec<String>,
    pub defs: Vec<Def>,
}

/// Escapes special characters for Markdown formatting
fn escape(text: &str) -> String {
    let special_chars = "_*[]()~`>#+-=|{}.!"; // Characters to escape
    text.chars()
        .flat_map(|c| {
            if special_chars.contains(c) {
                vec!['\\', c]
            } else {
                vec![c]
            }
        })
        .collect()
}

impl WordInfo {
    /// Prepares a formatted message with keyboard for display
    pub fn get_message(&self, def_idx: usize) -> (String, InlineKeyboardMarkup) {
        let def = &self.defs[def_idx];
        let message = format!(
            "{} *__{}__*\n{}",
            escape(&self.word),
            escape(&def.functional_label),
            escape(
                &def.definitions
                    .iter()
                    .enumerate()
                    .map(|(i, v)| format!("{}. {}", i + 1, v))
                    .collect::<Vec<String>>()
                    .join("\n")
            )
        );

        let buttons: Vec<_> = vec![("prev", def_idx.wrapping_sub(1)), ("next", def_idx + 1)]
            .into_iter()
            .filter_map(|(txt, idx)| {
                if idx < self.defs.len() {
                    Some(InlineKeyboardButton::callback(
                        txt,
                        format!("def_{}_{}", self.word, idx.to_string()),
                    ))
                } else {
                    None
                }
            })
            .collect();

        let keyboard = InlineKeyboardMarkup::new(vec![buttons]);

        (message, keyboard)
    }

    /// Sends a new message with word information
    pub async fn send_message(
        &self,
        bot: &Bot,
        chat_id: ChatId,
        def_idx: usize,
    ) -> ResponseResult<()> {
        let (message, keyboard) = self.get_message(def_idx);
        bot.send_message(chat_id, message)
            .reply_markup(keyboard)
            .parse_mode(MarkdownV2)
            .await?;

        Ok(())
    }

    /// Edits an existing message with word information
    pub async fn edit_message(
        &self,
        bot: &Bot,
        chat_id: ChatId,
        message_id: MessageId,
        def_idx: usize,
    ) -> ResponseResult<()> {
        let (message, keyboard) = self.get_message(def_idx);
        bot.edit_message_text(chat_id, message_id, message)
            .parse_mode(MarkdownV2)
            .await?;
        bot.edit_message_reply_markup(chat_id, message_id)
            .reply_markup(keyboard)
            .await?;
        Ok(())
    }
}

/// Word definition containing the functional label and definitions
#[derive(Encode, Decode, Clone, Debug)]
pub struct Def {
    pub definitions: Vec<String>,
    pub functional_label: String,
}

const CACHE_SIZE: u64 = 100_000;
const CACHE_PATH: &str = "cache.bin";
static CACHE: OnceLock<Cache<String, WordInfo>> = OnceLock::new();
static CLIENT: OnceLock<MerriamWebsterClient> = OnceLock::new();

/// Cache entry for serialization/deserialization
#[derive(Encode, Decode)]
struct CacheEntry {
    key: String,
    value: WordInfo,
}

/// Initializes the word cache from disk if available
pub async fn init_cache() {
    let cache: Cache<String, WordInfo> = Cache::new(CACHE_SIZE);

    if let Ok(file) = File::open(CACHE_PATH) {
        let reader = BufReader::new(file);
        let entries_result: Result<Vec<CacheEntry>, _> =
            bincode::decode_from_reader(reader, bincode::config::standard());

        match entries_result {
            Ok(entries) => {
                log::info!("Loaded {} entries from cache", entries.len());
                for entry in entries {
                    cache.insert(entry.key, entry.value).await;
                }
            }
            Err(e) => log::error!("Failed to load cache: {}", e),
        }
    } else {
        log::info!("No cache file found, starting with empty cache");
    }

    let _ = CACHE.set(cache);
}

/// Gets a reference to the global word cache
pub fn get_cache() -> &'static Cache<String, WordInfo> {
    CACHE
        .get()
        .expect("Cache not initialized. Call init_cache() first")
}

/// Initializes the Merriam-Webster API client
fn init_client() -> MerriamWebsterClient {
    let api_key = env::var("MERRIAM_WEBSTER_API_KEY")
        .expect("MERRIAM_WEBSTER_API_KEY environment variable not set");
    MerriamWebsterClient::new(api_key.into())
}

/// Gets a reference to the global Merriam-Webster API client
fn get_client() -> &'static MerriamWebsterClient {
    CLIENT.get_or_init(|| init_client())
}

/// Gets a random word that satisfies the given predicate, optionally starting with a specific character
pub async fn get_random_word<P>(
    predicate: P,
    start_char: Option<char>,
) -> Result<WordInfo, DictionaryError>
where
    P: Fn(&str) -> bool,
{
    let char = match start_char {
        Some(c) => c,
        None => ('a'..='z').choose(&mut rand::rng()).ok_or_else(|| {
            DictionaryError::ApiError("Failed to generate random character".to_string())
        })?,
    };

    let embeddings = get_embeddings()
        .map_err(|e| DictionaryError::ApiError(format!("Failed to get embeddings: {}", e)))?;

    let char_map = embeddings
        .get(&char)
        .ok_or_else(|| DictionaryError::NotFound(format!("No embeddings for letter '{}'", char)))?;

    let word = char_map
        .keys()
        .filter(|k| predicate(k))
        .choose(&mut rng())
        .ok_or_else(|| DictionaryError::NotFound("No matching word found".to_string()))?;

    get_word_details(word).await
}

/// Gets detailed information about a word
pub async fn get_word_details(word: &str) -> Result<WordInfo, DictionaryError> {
    let cache = get_cache();

    // Check cache first for efficiency
    if let Some(cached_word) = cache.get(word).await {
        return Ok(cached_word);
    }

    // Validate word existence
    if !is_valid_word(word) {
        return Err(DictionaryError::NotFound(format!(
            "'{}' is not in our wordlist",
            word
        )));
    }

    log::info!("Fetching details for word: {}", word);

    // Call API for word details
    let client = get_client();
    let def = client
        .collegiate_definition(word.into())
        .await
        .map_err(|_| DictionaryError::ApiError(format!("No definition found for '{}'", word)))?;

    // Process definitions
    let defs = def
        .iter()
        .filter_map(|d| {
            let definitions = d.shortdef.as_ref()?;
            Some(Def {
                functional_label: d.fl.clone().unwrap_or_default(),
                definitions: definitions.iter().map(|s| s.to_string()).collect(),
            })
        })
        .collect::<Vec<Def>>();

    if defs.is_empty() {
        return Err(DictionaryError::NotFound(format!(
            "No usable definitions for '{}'",
            word
        )));
    }

    // Collect word stems
    let stems = def.iter().flat_map(|d| d.meta.stems.clone()).collect();

    // Create and cache the word info
    let word_info = WordInfo {
        word: word.into(),
        stems,
        defs,
    };

    cache.insert(word.into(), word_info.clone()).await;

    Ok(word_info)
}

/// Saves the word cache to disk
pub fn save_cache(
    cache: &'static Cache<String, WordInfo>,
    file_path: &str,
) -> Result<(), DictionaryError> {
    log::info!("Saving cache to {}", file_path);

    let file = File::create(file_path)?;
    let mut writer = BufWriter::new(file);

    let data = cache
        .iter()
        .map(|(k, v)| CacheEntry {
            key: k.to_string(),
            value: v.clone(),
        })
        .collect::<Vec<_>>();

    bincode::encode_into_std_write(&data, &mut writer, bincode::config::standard())
        .map_err(|e| DictionaryError::CacheError(format!("Failed to encode cache: {}", e)))?;

    log::info!("Cache saved with {} entries", data.len());
    Ok(())
}
