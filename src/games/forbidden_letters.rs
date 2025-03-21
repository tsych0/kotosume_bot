use crate::command::Command;
use crate::dictionary::{get_random_word, WordInfo};
use crate::state::MyDialogue;
use crate::state::State::{ForbiddenLetters, Start};
use rand::prelude::IteratorRandom;
use rand::rng;
use teloxide::payloads::SendMessageSetters;
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

    let forbidden_letters = ('a'..'z').choose_multiple(&mut rng(), 1);

    loop {
        if let Ok(word) = get_random_word().await {
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
    todo!("Implementation pending!")
}
