use crate::command::Command;
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
pub async fn callback_handler(bot: Bot, q: CallbackQuery) -> ResponseResult<()> {
    if let Some(ref game) = q.data {
        let response = match game.as_str() {
            "word_chain" => "You selected Word Chain! Let’s start linking words.",
            "alphabet_sprint" => "Alphabet Sprint time! Ready to race through the letters?",
            "rhyme_time" => "Rhyme Time begins! Get those rhymes flowing.",
            "last_letter" => "Last Letter Scramble! Let’s twist those endings.",
            "synonym_string" => "Synonym String starts now! Link those meanings.",
            "word_ladder" => "Word Length Ladder! Climb up the word sizes.",
            "forbidden_letters" => "Forbidden Letters! Avoid the banned ones.",
            _ => "Unknown game selected!",
        };

        log::info!("You chose {game}");

        if let Some(Message { id, chat, .. }) = q.regular_message() {
            bot.send_message(chat.id, response).await?;
        }
    }

    bot.answer_callback_query(&q.id).await?;
    Ok(())
}

// Function to create the inline keyboard menu
fn make_game_menu() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    // List of games with their callback data
    let games = vec![
        ("Word Chain", "word_chain"),
        ("Alphabet Sprint", "alphabet_sprint"),
        ("Rhyme Time", "rhyme_time"),
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
