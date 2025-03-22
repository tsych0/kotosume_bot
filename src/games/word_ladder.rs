use crate::command::Command;
use crate::contains_any;
use crate::dictionary::{get_random_word, get_word_details, DictionaryError, WordInfo};
use crate::embeddings::{get_similar_word, EmbeddingError};
use crate::state::MyDialogue;
use crate::state::State::{Start, WordLengthLadder};
use log::{error, info, warn};
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult};
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

/// Error type specific to Word Ladder game
#[derive(Debug)]
enum WordLadderError {
    Dictionary(DictionaryError),
    Embedding(EmbeddingError),
    InvalidInput(String),
    NoValidWords(String),
}

impl From<DictionaryError> for WordLadderError {
    fn from(error: DictionaryError) -> Self {
        WordLadderError::Dictionary(error)
    }
}

impl From<EmbeddingError> for WordLadderError {
    fn from(error: EmbeddingError) -> Self {
        WordLadderError::Embedding(error)
    }
}

impl std::fmt::Display for WordLadderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WordLadderError::Dictionary(e) => write!(f, "Dictionary error: {}", e),
            WordLadderError::Embedding(e) => write!(f, "Embedding error: {}", e),
            WordLadderError::InvalidInput(msg) => write!(f, "{}", msg),
            WordLadderError::NoValidWords(msg) => write!(f, "{}", msg),
        }
    }
}

/// Start a new Word Ladder game
pub async fn start_word_ladder(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    info!("Starting Word Ladder game for chat {}", chat_id);

    bot.send_message(chat_id, "Word Length Ladder! Climb up the word sizes.")
        .await?;

    // Try to get a random word to start the game
    for _ in 0..3 {
        // Try up to 3 times
        match get_random_word(|w| w.len() == 2, None).await {
            Ok(word) => {
                let curr_char = match word.word.chars().last() {
                    Some(c) => c,
                    None => {
                        error!("Selected word '{}' has no characters", word.word);
                        bot.send_message(chat_id, "Error starting game, please try again.")
                            .await?;
                        return Ok(());
                    }
                };

                info!("Word Ladder started with word: {} (length 2)", word.word);

                bot.send_message(chat_id, format!("First word: {}", word.word))
                    .await?;
                word.send_message(&bot, chat_id, 0).await?;

                bot.send_message(
                    chat_id,
                    format!("Now give a word starting with '{}' of length 2", curr_char),
                )
                .await?;

                let _ = dialogue
                    .update(WordLengthLadder {
                        chain: vec![word],
                        curr_len: 2,
                        max_len: 8,
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

/// Handle player input during Word Ladder game
pub async fn word_ladder(
    bot: Bot,
    dialogue: MyDialogue,
    (chain, curr_len, max_len, curr_char): (Vec<WordInfo>, u8, u8, char),
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
                provide_hint(&bot, msg.chat.id, curr_char, curr_len).await?;
            }
            Ok(Command::Skip) => {
                skip_turn(
                    &bot,
                    msg.chat.id,
                    dialogue,
                    chain,
                    curr_len,
                    max_len,
                    curr_char,
                )
                .await?;
            }
            Ok(Command::Score) => {
                show_score(&bot, msg.chat.id, &chain, curr_len).await?;
            }
            Ok(Command::Rules) => {
                show_rules(&bot, msg.chat.id).await?;
            }
            Ok(Command::Stop) => {
                info!("Player stopped Word Ladder game in chat {}", msg.chat.id);

                // Show final score/summary
                let player_words = chain.len() / 2;
                let bot_words = chain.len() - player_words;
                let max_length_reached = if chain.is_empty() {
                    0
                } else {
                    chain.last().unwrap().word.len()
                };

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Game finished! Final score:\nYou: {} words\nBot: {} words\n\nMax word length reached: {}\n\nWords played: {}",
                        player_words,
                        bot_words,
                        max_length_reached,
                        chain.iter().map(|w| w.word.clone()).collect::<Vec<String>>().join(", ")
                    ),
                ).await?;

                bot.send_message(msg.chat.id, "Word Ladder game stopped. Thanks for playing!")
                    .await?;
                let _ = dialogue.update(Start).await;
            }
            Err(_) => {
                process_player_word(
                    text,
                    bot,
                    dialogue,
                    chain,
                    curr_len,
                    max_len,
                    curr_char,
                    msg.chat.id,
                )
                .await?
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
    curr_len: u8,
    max_len: u8,
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

    // Check if word starts with the current character and has correct length
    if !word.starts_with(curr_char) || word.len() != curr_len as usize {
        bot.send_message(
            chat_id,
            format!(
                "Your word must start with '{}' and be {} letters long",
                curr_char, curr_len
            ),
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

            // Check if we've reached the maximum word length
            if curr_len >= max_len {
                bot.send_message(
                    chat_id,
                    format!(
                        "Congratulations! You've reached the maximum length of {} letters!",
                        max_len
                    ),
                )
                .await?;
                let _ = dialogue.update(Start).await;
                return Ok(());
            }

            // Get the bot's response word (one letter longer)
            match get_bot_response(&word, &updated_stems, curr_len as usize + 1).await {
                Ok(next_word_details) => {
                    let next_char = match next_word_details.word.chars().last() {
                        Some(c) => c,
                        None => {
                            error!("Bot's word '{}' has no characters", next_word_details.word);
                            bot.send_message(chat_id, "Error in game, please try again.")
                                .await?;
                            let _ = dialogue.update(Start).await;
                            return Ok(());
                        }
                    };

                    chain.push(next_word_details.clone());
                    bot.send_message(chat_id, format!("My word: {}", next_word_details.word))
                        .await?;
                    next_word_details.send_message(&bot, chat_id, 0).await?;

                    // Prompt for the next word
                    bot.send_message(
                        chat_id,
                        format!(
                            "Now give a word starting with '{}' of length {}",
                            next_char,
                            curr_len as usize + 1
                        ),
                    )
                    .await?;

                    // Update game state
                    let _ = dialogue
                        .update(WordLengthLadder {
                            chain,
                            curr_len: curr_len + 1,
                            max_len,
                            curr_char: next_char,
                        })
                        .await;
                }
                Err(e) => {
                    error!("Failed to get bot response: {:?}", e);
                    bot.send_message(
                        chat_id,
                        "I can't think of a longer word! You win this round!",
                    )
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

/// Get the bot's response word with specific length
async fn get_bot_response(
    player_word: &str,
    used_words: &[String],
    target_length: usize,
) -> Result<WordInfo, WordLadderError> {
    let last_char = match player_word.chars().last() {
        Some(c) => c,
        None => {
            return Err(WordLadderError::InvalidInput(
                "Invalid player word".to_string(),
            ))
        }
    };

    // Get a similar word that hasn't been used
    let mut attempts = 0;
    const MAX_ATTEMPTS: usize = 3;

    while attempts < MAX_ATTEMPTS {
        attempts += 1;

        // Try to find a similar word
        let next_word_result = get_similar_word(player_word, last_char, |x| {
            !used_words.contains(&x.to_string()) && x.len() == target_length
        });

        match next_word_result {
            Ok(word) => {
                // Try to get details for this word
                match get_word_details(&word).await {
                    Ok(details) => return Ok(details),
                    Err(_) => continue, // Try another word
                }
            }
            Err(e) => {
                if attempts == MAX_ATTEMPTS {
                    return Err(WordLadderError::Embedding(e));
                }
                // Try again
            }
        }
    }

    Err(WordLadderError::NoValidWords(format!(
        "Could not find a valid word of length {}",
        target_length
    )))
}

/// Provide a hint for the current turn
async fn provide_hint(
    bot: &Bot,
    chat_id: ChatId,
    curr_char: char,
    curr_len: u8,
) -> ResponseResult<()> {
    info!("Providing hint for chat {}", chat_id);

    // Get a random word starting with the current character and with correct length
    match get_random_word(|w| w.len() == curr_len as usize, Some(curr_char)).await {
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
                format!("I can't think of a hint right now. Just try any word starting with '{}' that is {} letters long.", 
                      curr_char, curr_len),
            ).await?;
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
    curr_len: u8,
    max_len: u8,
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
    match get_random_word(|w| w.len() == curr_len as usize, Some(curr_char)).await {
        Ok(word) => {
            bot.send_message(chat_id, format!("My word: {}", word.word))
                .await?;
            word.send_message(bot, chat_id, 0).await?;
            chain.push(word.clone());

            // Get next word (one letter longer)
            match get_bot_response(&word.word, &used_stems, curr_len as usize + 1).await {
                Ok(next_word) => {
                    let next_char = match next_word.word.chars().last() {
                        Some(c) => c,
                        None => {
                            error!("Bot's word '{}' has no characters", next_word.word);
                            bot.send_message(chat_id, "Error in game, please try again.")
                                .await?;
                            let _ = dialogue.update(Start).await;
                            return Ok(());
                        }
                    };

                    chain.push(next_word.clone());
                    bot.send_message(
                        chat_id,
                        format!("And for the next word: {}", next_word.word),
                    )
                    .await?;
                    next_word.send_message(bot, chat_id, 0).await?;

                    bot.send_message(
                        chat_id,
                        format!(
                            "Now your turn. Give a word starting with '{}' of length {}",
                            next_char,
                            curr_len as usize + 1
                        ),
                    )
                    .await?;

                    let _ = dialogue
                        .update(WordLengthLadder {
                            chain,
                            curr_len: curr_len + 1,
                            max_len,
                            curr_char: next_char,
                        })
                        .await;
                }
                Err(e) => {
                    error!("Failed to get next word: {:?}", e);
                    bot.send_message(
                        chat_id,
                        "I can't think of a longer word! You win this round!",
                    )
                    .await?;
                    let _ = dialogue.update(Start).await;
                }
            }
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

/// Show the current score (word ladder progress)
async fn show_score(
    bot: &Bot,
    chat_id: ChatId,
    chain: &[WordInfo],
    curr_len: u8,
) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        format!("Current Progress:\nYou've reached word length: {}\nWords in ladder: {}\nKeep climbing!", 
            curr_len, chain.len()),
    ).await?;

    Ok(())
}

/// Show game rules
async fn show_rules(bot: &Bot, chat_id: ChatId) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        "Word Ladder Rules:\n\
        1. We start with a short word (2 letters)\n\
        2. Each new word must start with the last letter of the previous word\n\
        3. Word length increases by 1 with each turn\n\
        4. The goal is to reach a word of length 8\n\
        5. Use /hint for a hint, /skip to skip your turn, or /stop to end the game",
    )
    .await?;

    Ok(())
}
