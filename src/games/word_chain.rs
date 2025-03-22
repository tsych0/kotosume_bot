use crate::command::Command;
use crate::contains_any;
use crate::dictionary::{get_random_word, get_word_details, DictionaryError, WordInfo};
use crate::embeddings::{get_similar_word, EmbeddingError};
use crate::state::MyDialogue;
use crate::state::State::{Start, WordChain};
use log::{error, info, warn};
use teloxide::prelude::ResponseResult;
use teloxide::prelude::*;
use teloxide::types::{Me, Message};
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

/// Error type specific to Word Chain game
#[derive(Debug)]
enum WordChainError {
    Dictionary(DictionaryError),
    Embedding(EmbeddingError),
    InvalidInput(String),
    NoValidWords(String),
}

impl From<DictionaryError> for WordChainError {
    fn from(error: DictionaryError) -> Self {
        WordChainError::Dictionary(error)
    }
}

impl From<EmbeddingError> for WordChainError {
    fn from(error: EmbeddingError) -> Self {
        WordChainError::Embedding(error)
    }
}

impl std::fmt::Display for WordChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WordChainError::Dictionary(e) => write!(f, "Dictionary error: {}", e),
            WordChainError::Embedding(e) => write!(f, "Embedding error: {}", e),
            WordChainError::InvalidInput(msg) => write!(f, "{}", msg),
            WordChainError::NoValidWords(msg) => write!(f, "{}", msg),
        }
    }
}

/// Start a new Word Chain game
pub async fn start_word_chain(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    info!("Starting Word Chain game for chat {}", chat_id);

    bot.send_message(
        chat_id,
        "You selected Word Chain! Let's start linking words.",
    )
    .await?;

    // Try to get a random word to start the game
    for _ in 0..3 {
        // Try up to 3 times
        match get_random_word(|_| true, None).await {
            Ok(word) => {
                info!("Word Chain started with word: {}", word.word);

                // Get the last character of the word for the next word
                let curr_char = match word.word.chars().last() {
                    Some(c) => c,
                    None => {
                        error!("Selected word '{}' has no characters", word.word);
                        bot.send_message(chat_id, "Error starting game, please try again.")
                            .await?;
                        return Ok(());
                    }
                };

                // Send the first word
                bot.send_message(chat_id, format!("First word: {}", word.word))
                    .await?;
                word.send_message(&bot, chat_id, 0).await?;

                // Prompt user for the next word
                bot.send_message(
                    chat_id,
                    format!("Now give a word starting with '{}'", curr_char),
                )
                .await?;

                // Update dialogue state
                let _ = dialogue
                    .update(WordChain {
                        chain: vec![word],
                        curr_char,
                    })
                    .await;

                return Ok(());
            }
            Err(e) => {
                error!("Failed to get random word: {:?}", e);
                // Try again
            }
        }
    }

    // Failed after multiple attempts
    bot.send_message(
        chat_id,
        "Sorry, I'm having trouble starting the game. Please try again later.",
    )
    .await?;

    Ok(())
}

/// Handle player input during Word Chain game
pub async fn word_chain(
    bot: Bot,
    dialogue: MyDialogue,
    (chain, curr_char): (Vec<WordInfo>, char),
    msg: Message,
    me: Me,
) -> ResponseResult<()> {
    match msg.text() {
        Some(text) => match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) | Ok(Command::Play) | Ok(Command::Stats) => {
                bot.send_message(
                    msg.chat.id,
                    "Please stop this game first with /stop to use this command.",
                )
                .await?;
            }
            Ok(Command::Hint) => {
                provide_hint(&bot, msg.chat.id, curr_char, &chain).await?;
            }
            Ok(Command::Skip) => {
                skip_turn(&bot, msg.chat.id, dialogue, chain, curr_char).await?;
            }
            Ok(Command::Score) => {
                show_score(&bot, msg.chat.id, &chain).await?;
            }
            Ok(Command::Rules) => {
                show_rules(&bot, msg.chat.id).await?;
            }
            Ok(Command::Stop) => {
                info!("Player stopped Word Chain game in chat {}", msg.chat.id);

                // Show final score/summary
                let player_words = chain.len() / 2;
                let bot_words = chain.len() - player_words;

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Game finished! Final score:\nYou: {} words\nBot: {} words\n\nWord chain: {}",
                        player_words,
                        bot_words,
                        chain.iter().map(|w| w.word.clone()).collect::<Vec<String>>().join(" → ")
                    ),
                ).await?;

                bot.send_message(msg.chat.id, "Word Chain game stopped. Thanks for playing!")
                    .await?;
                let _ = dialogue.update(Start).await;
            }
            Err(_) => {
                process_player_word(text, bot, dialogue, chain, curr_char, msg.chat.id).await?;
            }
        },
        None => {
            // Ignore non-text messages
        }
    }
    Ok(())
}

/// Process a player's word submission
async fn process_player_word(
    text: &str,
    bot: Bot,
    dialogue: MyDialogue,
    mut chain: Vec<WordInfo>,
    curr_char: char,
    chat_id: ChatId,
) -> ResponseResult<()> {
    let words = text.split_whitespace().collect::<Vec<&str>>();

    // Check for valid input (single word)
    if words.is_empty() {
        bot.send_message(chat_id, "Please enter a word.").await?;
        return Ok(());
    }

    if words.len() > 1 {
        bot.send_message(chat_id, "Please enter only one word.")
            .await?;
        return Ok(());
    }

    let word = words[0].to_lowercase();

    // Check if word starts with the current character
    if !word.starts_with(curr_char) {
        bot.send_message(
            chat_id,
            format!("Your word must start with '{}'", curr_char),
        )
        .await?;
        return Ok(());
    }

    // Get list of already used words/stems
    let used_stems = chain
        .iter()
        .flat_map(|x| x.stems.clone())
        .collect::<Vec<String>>();

    // Validate the player's word
    match get_word_details(&word).await {
        Ok(word_details) => {
            // Check if word has already been used
            if contains_any(&used_stems, &word_details.stems) {
                bot.send_message(
                    chat_id,
                    "That word (or a form of it) has already been used.",
                )
                .await?;
                return Ok(());
            }

            // Add the player's word to the chain
            info!("Player used word: {} in chat {}", word, chat_id);
            let mut updated_stems = used_stems.clone();
            updated_stems.push(word.clone());

            word_details.send_message(&bot, chat_id, 0).await?;
            chain.push(word_details.clone());

            // Get the bot's response word
            match get_bot_response(&word, &updated_stems).await {
                Ok(next_word_details) => {
                    chain.push(next_word_details.clone());
                    bot.send_message(chat_id, format!("My word: {}", next_word_details.word))
                        .await?;
                    next_word_details.send_message(&bot, chat_id, 0).await?;

                    // Get the next character for the player's turn
                    let next_char = match next_word_details.word.chars().last() {
                        Some(c) => c,
                        None => {
                            error!("Bot word '{}' has no characters", next_word_details.word);
                            return Ok(());
                        }
                    };

                    // Prompt for the next word
                    bot.send_message(
                        chat_id,
                        format!("Now give a word starting with '{}'", next_char),
                    )
                    .await?;

                    // Update game state
                    let _ = dialogue
                        .update(WordChain {
                            chain,
                            curr_char: next_char,
                        })
                        .await;
                }
                Err(e) => {
                    error!("Failed to get bot response: {:?}", e);
                    bot.send_message(chat_id, "I can't think of a word! You win this round!")
                        .await?;
                    let _ = dialogue.update(Start).await;
                }
            }
        }
        Err(e) => {
            warn!(
                "Invalid word attempt '{}' in chat {}: {:?}",
                word, chat_id, e
            );
            bot.send_message(
                chat_id,
                format!("I don't recognize '{}'. Please try another word.", word),
            )
            .await?;
        }
    }

    Ok(())
}

/// Get the bot's response word
async fn get_bot_response(
    player_word: &str,
    used_words: &[String],
) -> Result<WordInfo, WordChainError> {
    let mut used_words = used_words.to_vec();

    // Get the last character of the player's word
    let last_char = player_word
        .chars()
        .last()
        .ok_or_else(|| WordChainError::InvalidInput("Player word has no characters".to_string()))?;

    // Get a similar word that hasn't been used
    let mut attempts = 0;
    const MAX_ATTEMPTS: usize = 3;

    while attempts < MAX_ATTEMPTS {
        attempts += 1;

        // Try to find a similar word
        let next_word_result = get_similar_word(player_word, last_char, |x| {
            !used_words.contains(&x.to_string())
        });

        match next_word_result {
            Ok(word) => {
                // Try to get details for this word
                match get_word_details(&word).await {
                    Ok(details) => {
                        if contains_any(&used_words, &details.stems) {
                            used_words.extend(details.stems.clone());
                            continue;
                        }
                        return Ok(details);
                    }
                    Err(_) => continue, // Try another word
                }
            }
            Err(e) => {
                if attempts == MAX_ATTEMPTS {
                    return Err(WordChainError::Embedding(e));
                }
                // Try again
            }
        }
    }

    Err(WordChainError::NoValidWords(format!(
        "Could not find a valid word starting with '{}'",
        last_char
    )))
}

/// Provide a hint for the current turn
async fn provide_hint(
    bot: &Bot,
    chat_id: ChatId,
    curr_char: char,
    chain: &[WordInfo],
) -> ResponseResult<()> {
    info!("Providing hint for chat {}", chat_id);

    let used_stems = chain
        .iter()
        .flat_map(|x| x.stems.clone())
        .collect::<Vec<String>>();

    // Get a random word starting with the current character (not used before)
    match get_random_word(|w| !used_stems.contains(&w.to_string()), Some(curr_char)).await {
        Ok(hint) => {
            bot.send_message(
                chat_id,
                format!(
                    "Hint: You could try a word like '{}' or something similar.",
                    hint.word
                ),
            )
            .await?;
        }
        Err(_) => {
            bot.send_message(
                chat_id,
                format!(
                    "I can't think of a hint right now. Just try any word starting with '{}'.",
                    curr_char
                ),
            )
            .await?;
        }
    }

    Ok(())
}

/// Skip the current turn
async fn skip_turn(
    bot: &Bot,
    chat_id: ChatId,
    dialogue: MyDialogue,
    mut chain: Vec<WordInfo>,
    curr_char: char,
) -> ResponseResult<()> {
    info!("Player skipped turn in chat {}", chat_id);

    bot.send_message(chat_id, "Skipping your turn...").await?;

    // Get list of used words
    let used_stems = chain
        .iter()
        .flat_map(|x| x.stems.clone())
        .collect::<Vec<String>>();

    // Try to get a word for the bot
    match get_random_word(|w| !used_stems.contains(&w.to_string()), Some(curr_char)).await {
        Ok(word) => {
            bot.send_message(chat_id, format!("My word: {}", word.word))
                .await?;
            word.send_message(bot, chat_id, 0).await?;
            chain.push(word.clone());

            // Get next character
            let next_char = word.word.chars().last().unwrap_or('a');

            bot.send_message(
                chat_id,
                format!("Now give a word starting with '{}'", next_char),
            )
            .await?;

            let _ = dialogue
                .update(WordChain {
                    chain,
                    curr_char: next_char,
                })
                .await;
        }
        Err(e) => {
            error!("Failed to get random word for skip: {:?}", e);
            bot.send_message(
                chat_id,
                "I can't think of a word either! Let's end this game.",
            )
            .await?;
            let _ = dialogue.update(Start).await;
        }
    }

    Ok(())
}

/// Show the current score (chain length)
async fn show_score(bot: &Bot, chat_id: ChatId, chain: &[WordInfo]) -> ResponseResult<()> {
    let player_words = chain.len() / 2;
    let bot_words = chain.len() - player_words;

    bot.send_message(
        chat_id,
        format!(
            "Current chain has {} words total.\nYou: {} words\nBot: {} words",
            chain.len(),
            player_words,
            bot_words
        ),
    )
    .await?;

    Ok(())
}

/// Show game rules
async fn show_rules(bot: &Bot, chat_id: ChatId) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        "Word Chain Rules:\n\
        1. I'll start with a word\n\
        2. You must respond with a word that starts with the last letter of my word\n\
        3. We take turns continuing the chain\n\
        4. No repeating words\n\
        5. Use /hint for a hint, /skip to skip your turn, or /stop to end the game",
    )
    .await?;

    Ok(())
}
