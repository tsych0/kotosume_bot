use crate::embeddings::{get_vocabulary, is_valid_word};
use bincode::{Decode, Encode};
use merriam_webster_http::MerriamWebsterClient;
use moka::future::Cache;
use rand::prelude::{IndexedRandom, IteratorRandom};
use rand::rng;
use std::env;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::OnceLock;

#[derive(Encode, Decode, Clone)]
pub struct WordInfo {
    pub word: String,
    pub stems: Vec<String>,
    pub defs: Vec<Def>,
}

impl Display for WordInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Word: {}", self.word)?;
        writeln!(
            f,
            "Definitions: \n {}",
            self.defs
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[derive(Encode, Decode, Clone)]
pub struct Def {
    pub definitions: Vec<String>,
    pub functional_label: String,
}

impl Display for Def {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for d in &self.definitions {
            writeln!(f, "({}): {}", self.functional_label, d)?;
        }
        Ok(())
    }
}

const CACHE_SIZE: u64 = 30_000;
const CACHE_PATH: &str = "cache.bin";
static CACHE: OnceLock<Cache<String, WordInfo>> = OnceLock::new();
static CLIENT: OnceLock<MerriamWebsterClient> = OnceLock::new();

#[derive(Encode, Decode)]
struct CacheEntry {
    key: String,
    value: WordInfo,
}

pub async fn init_cache() {
    let cache: Cache<String, WordInfo> = Cache::new(CACHE_SIZE);

    if let Ok(file) = File::open(CACHE_PATH) {
        let reader = BufReader::new(file);
        let entries_result: Result<Vec<CacheEntry>, _> =
            bincode::decode_from_reader(reader, bincode::config::standard());
        if let Ok(entries) = entries_result {
            for entry in entries {
                cache.insert(entry.key, entry.value).await;
            }
        }
    }

    let _ = CACHE.set(cache);
}

pub fn get_cache() -> &'static Cache<String, WordInfo> {
    CACHE.get().unwrap()
}

fn init_client() -> MerriamWebsterClient {
    let api_key = env::var("MERRIAM_WEBSTER_API_KEY").unwrap();
    MerriamWebsterClient::new(api_key.into())
}

fn get_client() -> &'static MerriamWebsterClient {
    CLIENT.get_or_init(|| init_client())
}

pub async fn get_random_word() -> Result<WordInfo, String> {
    let vocab = get_vocabulary();
    let word = vocab
        .choose(&mut rng())
        .map(|x| x.clone())
        .ok_or("cannot choose word")?;
    println!("getting word details of word {}", word);
    get_word_details(&word).await
}

pub async fn get_word_details(word: &str) -> Result<WordInfo, String> {
    let cache = get_cache();
    if cache.contains_key(word) {
        return cache
            .get(word)
            .await
            .clone()
            .ok_or("word not found in cache".into());
    }

    if !is_valid_word(word) {
        return Err(format!("{} is not in our wordlist.", word).into());
    }

    let client = get_client();
    let def = client
        .collegiate_definition(word.into())
        .await
        .map_err(|_| format!("No definition found for {word}"))?;

    let defs = def
        .iter()
        .map(|d| {
            d.shortdef
                .as_ref()
                .ok_or(format!("Definition not found for {word}"))
                .and_then(|s| Ok(s.iter().map(|s| s.to_string()).collect::<Vec<_>>()))
                .and_then(|s| {
                    Ok(Def {
                        functional_label: d.fl.clone().unwrap_or(String::new()),
                        definitions: s,
                    })
                })
        })
        .collect::<Result<Vec<Def>, String>>()?;

    let stems = def.iter().map(|d| d.meta.stems.clone()).flatten().collect();

    let word_info = WordInfo {
        word: word.into(),
        stems,
        defs,
    };

    cache.insert(word.into(), word_info.clone()).await;

    Ok(word_info)
}

pub fn save_cache(cache: &'static Cache<String, WordInfo>, file_path: &str) -> std::io::Result<()> {
    let file = File::create(file_path)?;
    let mut writer = BufWriter::new(file);
    let data = cache
        .iter()
        .map(|(k, v)| CacheEntry {
            key: k.to_string(),
            value: v.clone(),
        })
        .collect::<Vec<_>>();
    bincode::encode_into_std_write(&data, &mut writer, bincode::config::standard()).unwrap();
    Ok(())
}
