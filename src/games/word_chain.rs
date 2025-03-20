use crate::command::Command;
use crate::dictionary::{get_random_word, get_word_details, WordInfo};
use crate::embeddings::{get_similar_or_opposite_word, is_valid_word};
use crate::state::MyDialogue;
use crate::state::State::{Start, WordChain};
use rand::{rng, Rng};
use teloxide::prelude::ResponseResult;
use teloxide::prelude::*;
use teloxide::types::{Me, Message};
use teloxide::utils::command::{BotCommands, ParseError};
use teloxide::Bot;

pub async fn word_chain(
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

pub async fn start_word_chain(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        "You selected Word Chain! Let’s start linking words.",
    )
    .await?;

    if let Ok(word) = get_random_word().await {
        let word_details = get_word_details(&word.word).await.unwrap();
        bot.send_message(chat_id, format!("First word: {}", word.word))
            .await?;
        bot.send_message(chat_id, word_details.to_string()).await?;
        bot.send_message(
            chat_id,
            format!(
                "Now give a word starting with {}",
                word.word.chars().last().unwrap()
            ),
        )
        .await?;
        let _ = dialogue.update(WordChain { chain: vec![word] }).await;
    } else {
        bot.send_message(chat_id, "Some error occurred.").await?;
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
        let word = words[0];
        if is_valid_word(word) {
            if let Ok(word_details) = get_word_details(word).await {
                bot.send_message(chat_id, word_details.to_string()).await?;
                chain.push(word_details.clone());
                let words_in_chain = chain
                    .iter()
                    .map(|x| x.word.clone())
                    .collect::<Vec<String>>();
                let next_word = get_similar_or_opposite_word(
                    word,
                    |x| !words_in_chain.contains(&x.into()),
                    rng().random_bool(0.5),
                );
                let next_word_details = get_word_details(&next_word).await.unwrap();
                chain.push(next_word_details.clone());
                bot.send_message(chat_id, format!("Next word: {}", next_word))
                    .await?;
                bot.send_message(chat_id, next_word_details.to_string())
                    .await?;
                bot.send_message(
                    chat_id,
                    format!(
                        "Now give a word starting with {}",
                        next_word.chars().last().unwrap()
                    ),
                )
                .await?;
                let _ = dialogue.update(WordChain { chain }).await;
            } else {
                bot.send_message(chat_id, "Something went wrong.").await?;
            }
        } else {
            bot.send_message(chat_id, "Invalid word!").await?;
        }
    }

    Ok(())
}
