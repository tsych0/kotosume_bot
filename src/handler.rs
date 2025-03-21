use crate::command::Command;
use crate::dictionary::get_word_details;
use crate::games::alphabet_sprint::start_alphabet_sprint;
use crate::games::forbidden_letters::start_forbidden_letters;
use crate::games::scrambled::start_last_letter_scramble;
use crate::games::synonym_string::start_synonym_string;
use crate::games::word_chain::start_word_chain;
use crate::games::word_ladder::start_word_ladder;
use crate::state::MyDialogue;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{CallbackQuery, Message, Requester, ResponseResult};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, Me};
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

pub async fn message_handler(bot: Bot, msg: Message, me: Me) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) => {
                bot.send_message(msg.chat.id, "Welcome to the Wordplay Bot! Choose a game:")
                    .reply_markup(make_game_menu())
                    .await?;
            }
            Ok(Command::Play) => {}
            Ok(Command::Hint) => {}
            Ok(Command::Skip) => {}
            Ok(Command::Score) => {}
            Ok(Command::Rules) => {}
            Ok(Command::Stats) => {}
            Ok(Command::Stop) => {}

            Err(_) => {
                bot.send_message(msg.chat.id, "Command not found!").await?;
            }
        }
    }
    Ok(())
}

// Handler for callback queries (when a game is selected)
pub async fn callback_handler(
    bot: Bot,
    q: CallbackQuery,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    if let Some(ref game) = q.data {
        log::info!("You chose {game}");

        bot.answer_callback_query(&q.id).await?;
        if let Some(Message { id, chat, .. }) = q.regular_message() {
            match game.as_str() {
                "word_chain" => start_word_chain(chat.id, bot, dialogue).await,
                "alphabet_sprint" => start_alphabet_sprint(chat.id, bot, dialogue).await,
                "last_letter" => start_last_letter_scramble(chat.id, bot, dialogue).await,
                "synonym_string" => start_synonym_string(chat.id, bot, dialogue).await,
                "word_ladder" => start_word_ladder(chat.id, bot, dialogue).await,
                "forbidden_letters" => start_forbidden_letters(chat.id, bot, dialogue).await,
                s if s.starts_with("def") => {
                    let mut x = s.split("_");
                    let _ = x.next();
                    let word = x.next().unwrap();
                    let idx = x.next().unwrap().parse().unwrap();
                    let word_details = get_word_details(word).await.unwrap();
                    word_details
                        .edit_message(&bot, chat.id, id.clone(), idx)
                        .await?;
                    Ok(())
                }
                _ => Ok(()),
            }?;
        }
    }

    Ok(())
}

// Function to create the inline keyboard menu
fn make_game_menu() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    // List of games with their callback data
    let games = vec![
        ("Word Chain", "word_chain"),
        ("Alphabet Sprint", "alphabet_sprint"),
        ("Last Letter Scramble", "last_letter"),
        ("Synonym String", "synonym_string"),
        ("Word Length Ladder", "word_ladder"),
        ("Forbidden Letters", "forbidden_letters"),
    ];

    // Add buttons for each game (2 per row for better layout)
    for chunk in games.chunks(2) {
        let row = chunk
            .iter()
            .map(|(name, callback)| {
                InlineKeyboardButton::callback(name.to_string(), callback.to_string())
            })
            .collect();
        keyboard.push(row);
    }

    InlineKeyboardMarkup::new(keyboard)
}
