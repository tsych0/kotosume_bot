use crate::command::Command;
use crate::dictionary::{get_random_word, get_word_details, WordInfo};
use crate::embeddings::get_similar_word;
use crate::state::MyDialogue;
use crate::state::State::{RhymeTime, Start, WordChain};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::sync::OnceLock;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult};
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

static CMU_DICT_DATA: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();

pub async fn start_rhyme_time(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    bot.send_message(chat_id, "Rhyme Time begins! Get those rhymes flowing.")
        .await?;

    loop {
        if let Ok(word) = get_random_word().await {
            bot.send_message(chat_id, format!("First word: {}", word.word))
                .await?;
            word.send_message(&bot, chat_id, 0).await?;
            bot.send_message(
                chat_id,
                format!(
                    "Now give a word starting with '{}' that rhymes with '{}'",
                    word.word.chars().last().unwrap(),
                    word.word
                ),
            )
            .await?;
            let _ = dialogue.update(RhymeTime { chain: vec![word] }).await;
            return Ok(());
        }
    }
}

pub async fn rhyme_time(
    bot: Bot,
    dialogue: MyDialogue,
    chain: Vec<WordInfo>,
    msg: Message,
    me: Me,
) -> ResponseResult<()> {
    match msg.text() {
        Some(text) => match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) | Ok(Command::Play) | Ok(Command::Stats) => {
                bot.send_message(msg.chat.id, "Please stop this game to use this command.")
                    .await?;
            }
            Ok(Command::Hint) => {}
            Ok(Command::Skip) => {}
            Ok(Command::Score) => {}
            Ok(Command::Rules) => {}
            Ok(Command::Stop) => {
                let _ = dialogue.update(Start).await;
            }
            Err(_) => game(text, bot, dialogue, chain, msg.chat.id).await?,
        },
        None => {}
    }
    Ok(())
}

async fn game(
    text: &str,
    bot: Bot,
    dialogue: MyDialogue,
    mut chain: Vec<WordInfo>,
    chat_id: ChatId,
) -> ResponseResult<()> {
    let words = text.split_whitespace().collect::<Vec<&str>>();
    if words.len() > 1 {
        bot.send_message(chat_id, "Too many words.").await?;
    } else {
        let word = words[0].to_lowercase();

        let last_constraint = chain.last().unwrap().word.chars().last().unwrap();
        if !word.starts_with(last_constraint) {
            bot.send_message(
                chat_id,
                format!("Give word starting with '{}'", last_constraint),
            )
            .await?;
            return Ok(());
        }
        let mut chosen_words = chain
            .iter()
            .map(|x| x.stems.clone())
            .flatten()
            .collect::<Vec<String>>();

        if chosen_words.contains(&word) {
            bot.send_message(chat_id, "Word already used.").await?;
            return Ok(());
        }
        chosen_words.push(word.clone());

        match get_word_details(&word).await {
            Ok(word_details) => {
                word_details.send_message(&bot, chat_id, 0).await?;
                chain.push(word_details.clone());

                let mut next_word = String::new();
                let mut next_word_details = None;
                while next_word_details.is_none() {
                    next_word = get_similar_word(&word, word.chars().last().unwrap(), |x| {
                        !chosen_words.contains(&x.into()) && rhymes(x, &word)
                    });
                    chosen_words.push(next_word.clone());
                    next_word_details = get_word_details(&next_word).await.ok();
                }
                let next_word_details = next_word_details.unwrap();
                chain.push(next_word_details.clone());
                bot.send_message(chat_id, format!("Next word: {}", next_word))
                    .await?;
                next_word_details.send_message(&bot, chat_id, 0).await?;
                bot.send_message(
                    chat_id,
                    format!(
                        "Now give a word starting with '{}' and rhymes with {}",
                        next_word,
                        next_word.chars().last().unwrap()
                    ),
                )
                .await?;
                let _ = dialogue.update(WordChain { chain }).await;
            }
            Err(e) => {
                bot.send_message(chat_id, e).await?;
            }
        }
    }

    Ok(())
}

fn load_cmu_dict(filename: &str) -> HashMap<String, Vec<String>> {
    let file = File::open(filename).expect("Failed to open CMUdict file");
    let reader = io::BufReader::new(file);
    let mut dict = HashMap::new();

    for line in reader.lines().filter_map(Result::ok) {
        if line.starts_with(";;;") || line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() > 1 {
            let word = parts[0].to_lowercase();
            let phonemes = parts[1..].iter().map(|s| s.to_string()).collect();
            dict.insert(word, phonemes);
        }
    }
    dict
}

fn get_rhyme_suffix(
    word: &str,
    dict: &HashMap<String, Vec<String>>,
    length: usize,
) -> Option<Vec<String>> {
    dict.get(word)
        .map(|phonemes| phonemes.iter().rev().take(length).cloned().collect())
}

fn rhymes(word1: &str, word2: &str) -> bool {
    let dict = CMU_DICT_DATA.get_or_init(|| load_cmu_dict("cmudict.txt"));
    match (
        get_rhyme_suffix(word1, dict, 3),
        get_rhyme_suffix(word2, dict, 3),
    ) {
        (Some(suffix1), Some(suffix2)) => suffix1 == suffix2,
        _ => false,
    }
}
