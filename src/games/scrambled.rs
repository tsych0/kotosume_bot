use crate::command::Command;
use crate::contains_any;
use crate::dictionary::{get_random_word, get_word_details, WordInfo};
use crate::embeddings::get_similar_word;
use crate::state::MyDialogue;
use crate::state::State::{LastLetterScramble, Start};
use std::collections::HashSet;
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult};
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

pub async fn start_last_letter_scramble(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    bot.send_message(chat_id, "Last Letter Scramble! Let’s twist those endings.")
        .await?;

    loop {
        if let Ok(word) = get_random_word(|_| true).await {
            bot.send_message(chat_id, format!("First word: {}", word.word))
                .await?;
            word.send_message(&bot, chat_id, 0).await?;
            let curr_char = word.word.chars().last().unwrap();
            bot.send_message(
                chat_id,
                format!("Now give a word starting with '{}'", curr_char),
            )
            .await?;
            let _ = dialogue
                .update(LastLetterScramble {
                    chain: vec![word],
                    level: 3,
                    curr_char,
                })
                .await;
            return Ok(());
        }
    }
}

pub async fn last_letter_scramble(
    bot: Bot,
    dialogue: MyDialogue,
    (chain, level): (Vec<WordInfo>, u8),
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
                bot.send_message(msg.chat.id, "Game stopped!").await?;
            }
            Err(_) => game(text, bot, dialogue, chain, level, msg.chat.id).await?,
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
    level: u8,
    chat_id: ChatId,
) -> ResponseResult<()> {
    let words = text.split_whitespace().collect::<Vec<&str>>();
    if words.len() > 1 {
        bot.send_message(chat_id, "Too many words.").await?;
    } else {
        let word = words[0].to_lowercase();
        let prev_word = chain.last().unwrap();

        let last_constraint = prev_word.word.chars().last().unwrap();
        if !word.starts_with(last_constraint)
            && contains_at_least_n_chars(&word, &prev_word.word, level as usize)
        {
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

        match get_word_details(&word).await {
            Ok(word_details) => {
                if contains_any(&chosen_words, &word_details.stems) {
                    bot.send_message(chat_id, "Word already used.").await?;
                    return Ok(());
                }
                chosen_words.push(word.clone());

                word_details.send_message(&bot, chat_id, 0).await?;
                chain.push(word_details.clone());

                let mut next_word = String::new();
                let mut next_word_details = None;
                while next_word_details.is_none() {
                    next_word = get_similar_word(&word, word.chars().last().unwrap(), |x| {
                        !chosen_words.contains(&x.into())
                            && contains_at_least_n_chars(&word, x, level as usize)
                    });
                    chosen_words.push(next_word.clone());
                    next_word_details = get_word_details(&next_word).await.ok();
                }
                let next_word_details = next_word_details.unwrap();
                chain.push(next_word_details.clone());
                bot.send_message(chat_id, format!("Next word: {}", next_word))
                    .await?;
                next_word_details.send_message(&bot, chat_id, 0).await?;

                let next_char = next_word.chars().last().unwrap();
                bot.send_message(
                    chat_id,
                    format!(
                        "Now give a word starting with '{}' that contains atleast {} letter from {}",
                        next_char,
                        level,
                        next_word
                    ),
                )
                    .await?;
                let _ = dialogue
                    .update(LastLetterScramble {
                        chain,
                        level,
                        curr_char: next_char,
                    })
                    .await;
            }
            Err(e) => {
                bot.send_message(chat_id, e).await?;
            }
        }
    }

    Ok(())
}

fn contains_at_least_n_chars(chars: &str, s: &str, n: usize) -> bool {
    let char_set: HashSet<_> = chars.chars().collect();
    let mut found = HashSet::new();

    for c in s.chars() {
        if char_set.contains(&c) {
            found.insert(c);
            if found.len() >= n {
                return true;
            }
        }
    }
    false
}
