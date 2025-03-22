use crate::command::Command;
use crate::contains_any;
use crate::dictionary::{get_random_word, get_word_details, DictionaryError, WordInfo};
use crate::embeddings::{get_similar_word, EmbeddingError};
use crate::state::MyDialogue;
use crate::state::State::{LastLetterScramble, Start};
use log::{error, info, warn};
use std::collections::HashSet;
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult};
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

/// Error type specific to Last Letter Scramble game
#[derive(Debug)]
enum ScrambledError {
    Dictionary(DictionaryError),
    Embedding(EmbeddingError),
    InvalidInput(String),
    NoValidWords(String),
}

impl From<DictionaryError> for ScrambledError {
    fn from(error: DictionaryError) -> Self {
        ScrambledError::Dictionary(error)
    }
}

impl From<EmbeddingError> for ScrambledError {
    fn from(error: EmbeddingError) -> Self {
        ScrambledError::Embedding(error)
    }
}

impl std::fmt::Display for ScrambledError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ScrambledError::Dictionary(e) => write!(f, "Dictionary error: {}", e),
            ScrambledError::Embedding(e) => write!(f, "Embedding error: {}", e),
            ScrambledError::InvalidInput(msg) => write!(f, "{}", msg),
            ScrambledError::NoValidWords(msg) => write!(f, "{}", msg),
        }
    }
}

/// Start a new Last Letter Scramble game
pub async fn start_last_letter_scramble(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    info!("Starting Last Letter Scramble game for chat {}", chat_id);

    bot.send_message(chat_id, "Last Letter Scramble! Let's twist those endings.")
        .await?;

    // Try to get a random word to start the game
    for _ in 0..3 {
        // Try up to 3 times
        match get_random_word(|_| true).await {
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

                info!("Last Letter Scramble started with word: {}", word.word);

                bot.send_message(chat_id, format!("First word: {}", word.word))
                    .await?;
                word.send_message(&bot, chat_id, 0).await?;

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

/// Handle player input during Last Letter Scramble game
pub async fn last_letter_scramble(
    bot: Bot,
    dialogue: MyDialogue,
    (chain, level): (Vec<WordInfo>, u8),
    msg: Message,
    me: Me,
) -> ResponseResult<()> {
    let curr_char = match chain.last() {
        Some(word) => match word.word.chars().last() {
            Some(c) => c,
            None => {
                error!("Last word '{}' has no characters", word.word);
                bot.send_message(msg.chat.id, "Game error - please restart")
                    .await?;
                let _ = dialogue.update(Start).await;
                return Ok(());
            }
        },
        None => {
            error!("Chain is empty in last_letter_scramble");
            bot.send_message(msg.chat.id, "Game error - please restart")
                .await?;
            let _ = dialogue.update(Start).await;
            return Ok(());
        }
    };

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
                provide_hint(&bot, msg.chat.id, curr_char, level, &chain).await?;
            }
            Ok(Command::Skip) => {
                skip_turn(&bot, msg.chat.id, dialogue, chain, level, curr_char).await?;
            }
            Ok(Command::Score) => {
                show_score(&bot, msg.chat.id, &chain).await?;
            }
            Ok(Command::Rules) => {
                show_rules(&bot, msg.chat.id, level).await?;
            }
            Ok(Command::Stop) => {
                info!(
                    "Player stopped Last Letter Scramble game in chat {}",
                    msg.chat.id
                );
                bot.send_message(
                    msg.chat.id,
                    "Last Letter Scramble game stopped. Thanks for playing!",
                )
                .await?;
                let _ = dialogue.update(Start).await;
            }
            Err(_) => {
                process_player_word(text, bot, dialogue, chain, level, curr_char, msg.chat.id)
                    .await?;
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
    level: u8,
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
    let prev_word = match chain.last() {
        Some(w) => w,
        None => {
            error!("Chain is empty when processing player word");
            bot.send_message(chat_id, "Game error - please restart")
                .await?;
            let _ = dialogue.update(Start).await;
            return Ok(());
        }
    };

    // Check if word starts with the last letter of previous word
    // and contains at least N characters from the previous word
    if !word.starts_with(curr_char) {
        bot.send_message(
            chat_id,
            format!("Your word must start with '{}'", curr_char),
        )
        .await?;
        return Ok(());
    }

    if !contains_at_least_n_chars(&word, &prev_word.word, level as usize) {
        bot.send_message(
            chat_id,
            format!(
                "Your word must contain at least {} letter(s) from '{}'",
                level, prev_word.word
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

            // Get the bot's response word
            match get_bot_response(&word, &updated_stems, level).await {
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
                            "Now give a word starting with '{}' that contains at least {} letter(s) from '{}'",
                            next_char,
                            level,
                            next_word_details.word
                        ),
                    ).await?;

                    // Update game state
                    let _ = dialogue
                        .update(LastLetterScramble {
                            chain,
                            level,
                            curr_char: next_char,
                        })
                        .await;
                }
                Err(e) => {
                    error!("Failed to get bot response: {:?}", e);
                    bot.send_message(
                        chat_id,
                        "I can't think of a word that meets the criteria! You win this round!",
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

/// Get the bot's response word with specific letter constraints
async fn get_bot_response(
    player_word: &str,
    used_words: &[String],
    level: u8,
) -> Result<WordInfo, ScrambledError> {
    let last_char = match player_word.chars().last() {
        Some(c) => c,
        None => {
            return Err(ScrambledError::InvalidInput(
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
            !used_words.contains(&x.to_string())
                && contains_at_least_n_chars(player_word, x, level as usize)
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
                    return Err(ScrambledError::Embedding(e));
                }
                // Try again
            }
        }
    }

    Err(ScrambledError::NoValidWords(format!(
        "Could not find a valid word that contains {} letters from '{}'",
        level, player_word
    )))
}

/// Provide a hint for the current turn
async fn provide_hint(
    bot: &Bot,
    chat_id: ChatId,
    curr_char: char,
    level: u8,
    chain: &[WordInfo],
) -> ResponseResult<()> {
    info!("Providing hint for chat {}", chat_id);

    let prev_word = match chain.last() {
        Some(w) => &w.word,
        None => {
            error!("Chain is empty when providing hint");
            bot.send_message(chat_id, "Game error - please restart")
                .await?;
            return Ok(());
        }
    };

    let used_stems = chain
        .iter()
        .flat_map(|x| x.stems.clone())
        .collect::<Vec<String>>();

    // Try to find a word starting with current letter and containing required letters
    match get_random_word(|w| {
        w.starts_with(curr_char)
            && contains_at_least_n_chars(w, prev_word, level as usize)
            && !used_stems.contains(&w.to_string())
    })
    .await
    {
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
                    "I can't think of a hint right now. Just try a word starting with '{}' that contains at least {} letter(s) from '{}'.",
                    curr_char, level, prev_word
                ),
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
    level: u8,
    curr_char: char,
) -> ResponseResult<()> {
    info!("Player skipped turn in chat {}", chat_id);

    bot.send_message(chat_id, "Skipping your turn...").await?;

    // Get list of used words
    let used_stems = chain
        .iter()
        .flat_map(|x| x.stems.clone())
        .collect::<Vec<String>>();

    let prev_word = match chain.last() {
        Some(w) => &w.word,
        None => {
            error!("Chain is empty when skipping turn");
            bot.send_message(chat_id, "Game error - please restart")
                .await?;
            return Ok(());
        }
    };

    // Try to get a word for the bot
    match get_random_word(|w| {
        w.starts_with(curr_char)
            && contains_at_least_n_chars(w, prev_word, level as usize)
            && !used_stems.contains(&w.to_string())
    })
    .await
    {
        Ok(word) => {
            bot.send_message(chat_id, format!("My word: {}", word.word))
                .await?;
            word.send_message(bot, chat_id, 0).await?;

            let next_char = match word.word.chars().last() {
                Some(c) => c,
                None => {
                    error!("Bot's word '{}' has no characters", word.word);
                    bot.send_message(chat_id, "Error in game, please try again.")
                        .await?;
                    let _ = dialogue.update(Start).await;
                    return Ok(());
                }
            };

            chain.push(word.clone());

            bot.send_message(
                chat_id,
                format!(
                    "Now your turn. Give a word starting with '{}' that contains at least {} letter(s) from '{}'",
                    next_char, level, word.word
                ),
            ).await?;

            let _ = dialogue
                .update(LastLetterScramble {
                    chain,
                    level,
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

/// Show the current score (word count)
async fn show_score(bot: &Bot, chat_id: ChatId, chain: &[WordInfo]) -> ResponseResult<()> {
    let player_words = chain.len() / 2;
    let bot_words = chain.len() - player_words;

    bot.send_message(
        chat_id,
        format!(
            "Current score:\nYou: {} words\nBot: {} words",
            player_words, bot_words
        ),
    )
    .await?;

    Ok(())
}

/// Show game rules
async fn show_rules(bot: &Bot, chat_id: ChatId, level: u8) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        format!(
            "Last Letter Scramble Rules:\n\
            1. Each word must start with the last letter of the previous word\n\
            2. Each word must contain at least {} letter(s) from the previous word\n\
            3. No repeating words\n\
            4. Use /hint for a hint, /skip to skip your turn, or /stop to end the game",
            level
        ),
    )
    .await?;

    Ok(())
}

/// Check if string contains at least n characters from another string
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
