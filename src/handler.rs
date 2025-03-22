use crate::command::Command;
use crate::dictionary::get_word_details;
use crate::games::alphabet_sprint::start_alphabet_sprint;
use crate::games::forbidden_letters::start_forbidden_letters;
use crate::games::scrambled::start_last_letter_scramble;
use crate::games::synonym_string::start_synonym_string;
use crate::games::word_chain::start_word_chain;
use crate::games::word_ladder::start_word_ladder;
use crate::state::MyDialogue;
use log::{error, info, warn};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{CallbackQuery, Message, Requester, ResponseResult};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, Me};
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

/// Enum of callback data types for better type safety
pub enum CallbackType<'a> {
    GameSelect(&'a str),
    Definition { word: &'a str, index: usize },
    Unknown(&'a str),
}

/// Parse callback data into a structured type
fn parse_callback(data: &str) -> CallbackType {
    if data.starts_with("def_") {
        let parts: Vec<&str> = data.split('_').collect();
        if parts.len() >= 3 {
            if let Ok(index) = parts[2].parse::<usize>() {
                return CallbackType::Definition {
                    word: parts[1],
                    index,
                };
            }
        }
        CallbackType::Unknown(data)
    } else {
        // Game selection or other callback
        match data {
            "word_chain" | "alphabet_sprint" | "last_letter" | "synonym_string" | "word_ladder"
            | "forbidden_letters" => CallbackType::GameSelect(data),
            _ => CallbackType::Unknown(data),
        }
    }
}

/// Handle incoming text messages
pub async fn message_handler(bot: Bot, msg: Message, me: Me) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        info!("Received message: {}", text);

        match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) => {
                info!("Start command received from user {}", msg.chat.id);
                handle_start_command(&bot, msg.chat.id).await?;
            }
            Ok(Command::Play) => {
                info!("Play command not implemented yet");
                // TODO: Implement random game selection
            }
            Ok(Command::Hint) => {
                info!("Hint command not implemented yet");
                // TODO: Implement hint functionality
            }
            Ok(Command::Skip) => {
                info!("Skip command not implemented yet");
                // TODO: Implement skip functionality
            }
            Ok(Command::Score) => {
                info!("Score command not implemented yet");
                // TODO: Implement score tracking
            }
            Ok(Command::Rules) => {
                info!("Rules command not implemented yet");
                // TODO: Implement rules display
            }
            Ok(Command::Stats) => {
                info!("Stats command not implemented yet");
                // TODO: Implement statistics tracking
            }
            Ok(Command::Stop) => {
                info!("Stop command not implemented yet");
                // TODO: Implement game stopping
            }
            Err(_) => {
                warn!("Unknown command received: {}", text);
                bot.send_message(
                    msg.chat.id,
                    "Command not found! Try /start to see available commands.",
                )
                .await?;
            }
        }
    }
    Ok(())
}

/// Handle the start command
async fn handle_start_command(bot: &Bot, chat_id: teloxide::types::ChatId) -> ResponseResult<()> {
    bot.send_message(chat_id, "Welcome to the Wordplay Bot! Choose a game:")
        .reply_markup(make_game_menu())
        .await?;
    Ok(())
}

/// Handler for callback queries (when a game is selected or definition navigation)
pub async fn callback_handler(
    bot: Bot,
    q: CallbackQuery,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    if let Some(data) = &q.data {
        info!("Received callback: {}", data);

        // Always acknowledge the callback query to stop the loading indicator
        bot.answer_callback_query(&q.id).await?;

        if let Some(msg) = q.regular_message() {
            let chat_id = msg.chat.id;

            match parse_callback(data) {
                CallbackType::GameSelect(game) => {
                    info!("User selected game: {}", game);
                    handle_game_selection(game, chat_id, bot.clone(), dialogue).await?;
                }
                CallbackType::Definition { word, index } => {
                    info!(
                        "User navigating definition for '{}' to index {}",
                        word, index
                    );
                    handle_definition_navigation(word, index, &bot, chat_id, msg.id).await?;
                }
                CallbackType::Unknown(data) => {
                    warn!("Unknown callback data received: {}", data);
                }
            }
        }
    }

    Ok(())
}

/// Handle game selection from the menu
async fn handle_game_selection(
    game: &str,
    chat_id: teloxide::types::ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    match game {
        "word_chain" => start_word_chain(chat_id, bot, dialogue).await,
        "alphabet_sprint" => start_alphabet_sprint(chat_id, bot, dialogue).await,
        "last_letter" => start_last_letter_scramble(chat_id, bot, dialogue).await,
        "synonym_string" => start_synonym_string(chat_id, bot, dialogue).await,
        "word_ladder" => start_word_ladder(chat_id, bot, dialogue).await,
        "forbidden_letters" => start_forbidden_letters(chat_id, bot, dialogue).await,
        _ => {
            warn!("Unrecognized game selection: {}", game);
            Ok(())
        }
    }
}

/// Handle definition navigation for word details
async fn handle_definition_navigation(
    word: &str,
    index: usize,
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
) -> ResponseResult<()> {
    match get_word_details(word).await {
        Ok(word_details) => {
            word_details
                .edit_message(bot, chat_id, message_id, index)
                .await?;
            Ok(())
        }
        Err(e) => {
            error!("Error retrieving word details for '{}': {:?}", word, e);
            bot.send_message(
                chat_id,
                format!("Sorry, I couldn't find information for the word '{}'", word),
            )
            .await?;
            Ok(())
        }
    }
}

/// Create the inline keyboard menu with game choices
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
