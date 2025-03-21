use crate::command::Command;
use crate::dictionary::{get_random_word, WordInfo};
use crate::state::MyDialogue;
use crate::state::State::{AlphabetSprint, Start, WordChain};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult};
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

pub async fn start_alphabet_sprint(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        "Alphabet Sprint time! Ready to race through the letters?",
    )
    .await?;

    loop {
        if let Ok(word) = get_random_word().await {
            bot.send_message(chat_id, format!("First word: {}", word.word))
                .await?;
            word.send_message(&bot, chat_id, 0).await?;
            bot.send_message(
                chat_id,
                format!(
                    "Now give a word starting with '{}'",
                    word.word.chars().next().unwrap()
                ),
            )
            .await?;
            let _ = dialogue
                .update(AlphabetSprint {
                    words: vec![word.clone()],
                    alphabet: word.word.chars().next().unwrap(),
                })
                .await;
            return Ok(());
        }
    }
}

pub async fn alphabet_sprint(
    bot: Bot,
    dialogue: MyDialogue,
    (words, alphabet): (Vec<WordInfo>, char),
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
            Err(_) => game(text, bot, dialogue, words, alphabet, msg.chat.id).await?,
        },
        None => {}
    }
    Ok(())
}

async fn game(
    text: &str,
    bot: Bot,
    dialogue: MyDialogue,
    mut words: Vec<WordInfo>,
    alphabet: char,
    chat_id: ChatId,
) -> ResponseResult<()> {
    Ok(())
}
