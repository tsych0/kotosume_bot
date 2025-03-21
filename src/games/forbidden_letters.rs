use crate::command::Command;
use crate::contains_any;
use crate::dictionary::{get_random_word, get_word_details, WordInfo};
use crate::embeddings::get_similar_word;
use crate::state::MyDialogue;
use crate::state::State::{ForbiddenLetters, Start};
use rand::prelude::IteratorRandom;
use rand::rng;
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult};
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

pub async fn start_forbidden_letters(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    bot.send_message(chat_id, "Forbidden Letters! Avoid the banned ones.")
        .await?;

    let forbidden_letters = ('a'..='z').choose_multiple(&mut rng(), 1);

    loop {
        if let Ok(word) = get_random_word(|w| !w.contains(&forbidden_letters[..])).await {
            bot.send_message(
                chat_id,
                format!("Forbidden Letters! Avoid {:?}", forbidden_letters),
            )
            .await?;
            bot.send_message(chat_id, format!("First word: {}", word.word))
                .await?;
            word.send_message(&bot, chat_id, 0).await?;
            bot.send_message(
                chat_id,
                format!(
                    "Now give a word starting with '{}'",
                    word.word.chars().last().unwrap()
                ),
            )
            .await?;

            let _ = dialogue
                .update(ForbiddenLetters {
                    chain: vec![word],
                    forbidden_letters,
                })
                .await;
            return Ok(());
        }
    }
}

pub async fn forbidden_letters(
    bot: Bot,
    dialogue: MyDialogue,
    (chain, forbidden_letters): (Vec<WordInfo>, Vec<char>),
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
            Err(_) => game(text, bot, dialogue, chain, forbidden_letters, msg.chat.id).await?,
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
    forbidden_letters: Vec<char>,
    chat_id: ChatId,
) -> ResponseResult<()> {
    let words = text.split_whitespace().collect::<Vec<&str>>();
    if words.len() > 1 {
        bot.send_message(chat_id, "Too many words.").await?;
    } else {
        let word = words[0].to_lowercase();

        let last_constraint = chain.last().unwrap().word.chars().last().unwrap();
        if !word.starts_with(last_constraint) && !word.contains(&forbidden_letters[..]) {
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
                        !chosen_words.contains(&x.into()) && !x.contains(&forbidden_letters[..])
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
                        "Now give a word starting with '{}'",
                        next_word.chars().last().unwrap()
                    ),
                )
                .await?;
                let _ = dialogue
                    .update(ForbiddenLetters {
                        chain,
                        forbidden_letters,
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
