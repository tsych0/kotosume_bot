use crate::command::Command;
use crate::contains_any;
use crate::dictionary::{get_random_word, get_word_details, DictionaryError, WordInfo};
use crate::embeddings::{get_similar_word, EmbeddingError};
use crate::state::MyDialogue;
use crate::state::State::{ForbiddenLetters, Start};
use log::{error, info, warn};
use rand::prelude::IteratorRandom;
use rand::rng;
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult};
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

/// Error type specific to Forbidden Letters game
#[derive(Debug)]
enum ForbiddenLettersError {
    Dictionary(DictionaryError),
    Embedding(EmbeddingError),
    InvalidInput(String),
    NoValidWords(String),
}

impl From<DictionaryError> for ForbiddenLettersError {
    fn from(error: DictionaryError) -> Self {
        ForbiddenLettersError::Dictionary(error)
    }
}

impl From<EmbeddingError> for ForbiddenLettersError {
    fn from(error: EmbeddingError) -> Self {
        ForbiddenLettersError::Embedding(error)
    }
}

impl std::fmt::Display for ForbiddenLettersError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ForbiddenLettersError::Dictionary(e) => write!(f, "Dictionary error: {}", e),
            ForbiddenLettersError::Embedding(e) => write!(f, "Embedding error: {}", e),
            ForbiddenLettersError::InvalidInput(msg) => write!(f, "{}", msg),
            ForbiddenLettersError::NoValidWords(msg) => write!(f, "{}", msg),
        }
    }
}

/// Start a new Forbidden Letters game
pub async fn start_forbidden_letters(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    info!("Starting Forbidden Letters game for chat {}", chat_id);

    bot.send_message(chat_id, "Forbidden Letters! Avoid the banned ones.")
        .await?;

    // Choose some random letters to forbid
    let forbidden_letters = ('a'..='z').choose_multiple(&mut rng(), 1);

    info!(
        "Forbidden letters for chat {}: {:?}",
        chat_id, forbidden_letters
    );

    // Try to get a random word to start the game
    for _ in 0..3 {
        // Try up to 3 times
        match get_random_word(|w| !contains_forbidden_chars(w, &forbidden_letters), None).await {
            Ok(word) => {
                let next_char = match word.word.chars().last() {
                    Some(c) => c,
                    None => {
                        error!("Selected word '{}' has no characters", word.word);
                        bot.send_message(chat_id, "Error starting game, please try again.")
                            .await?;
                        return Ok(());
                    }
                };

                info!("Forbidden Letters started with word: {}", word.word);

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
                        "Now give a word starting with '{}' that doesn't contain forbidden letters",
                        next_char
                    ),
                )
                .await?;

                let _ = dialogue
                    .update(ForbiddenLetters {
                        chain: vec![word],
                        forbidden_letters: forbidden_letters.clone(),
                        curr_char: next_char,
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

/// Handle player input during Forbidden Letters game
pub async fn forbidden_letters(
    bot: Bot,
    dialogue: MyDialogue,
    (forbidden_letters, chain, curr_char): (Vec<char>, Vec<WordInfo>, char),
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
                provide_hint(&bot, msg.chat.id, curr_char, &forbidden_letters).await?;
            }
            Ok(Command::Skip) => {
                skip_turn(
                    &bot,
                    msg.chat.id,
                    dialogue,
                    chain,
                    forbidden_letters,
                    curr_char,
                )
                .await?;
            }
            Ok(Command::Score) => {
                show_score(&bot, msg.chat.id, &chain).await?;
            }
            Ok(Command::Rules) => {
                show_rules(&bot, msg.chat.id, &forbidden_letters).await?;
            }
            Ok(Command::Stop) => {
                info!(
                    "Player stopped Forbidden Letters game in chat {}",
                    msg.chat.id
                );

                // Show final score/summary
                let player_words = chain.len() / 2;
                let bot_words = chain.len() - player_words;

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Game finished! Final score:\nYou: {} words\nBot: {} words\n\nForbidden letters: {:?}\n\nWords played: {}",
                        player_words,
                        bot_words,
                        forbidden_letters,
                        chain.iter().map(|w| w.word.clone()).collect::<Vec<String>>().join(", ")
                    ),
                ).await?;

                bot.send_message(
                    msg.chat.id,
                    "Forbidden Letters game stopped. Thanks for playing!",
                )
                .await?;
                let _ = dialogue.update(Start).await;
            }
            Err(_) => {
                process_player_word(
                    text,
                    bot,
                    dialogue,
                    chain,
                    forbidden_letters,
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
    forbidden_letters: Vec<char>,
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

    // Check if word starts with correct letter and doesn't contain forbidden letters
    if !word.starts_with(curr_char) {
        bot.send_message(
            chat_id,
            format!("Your word must start with '{}'", curr_char),
        )
        .await?;
        return Ok(());
    }

    if contains_forbidden_chars(&word, &forbidden_letters) {
        bot.send_message(
            chat_id,
            format!(
                "Your word contains forbidden letters: {:?}",
                forbidden_letters
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
            match get_bot_response(&word, &updated_stems, &forbidden_letters).await {
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
                        format!("Now give a word starting with '{}'", next_char),
                    )
                    .await?;

                    // Update game state
                    let _ = dialogue
                        .update(ForbiddenLetters {
                            chain,
                            forbidden_letters,
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

/// Get the bot's response word that doesn't use forbidden letters
async fn get_bot_response(
    player_word: &str,
    used_words: &[String],
    forbidden_letters: &[char],
) -> Result<WordInfo, ForbiddenLettersError> {
    let mut used_words = used_words.to_vec();
    let last_char = match player_word.chars().last() {
        Some(c) => c,
        None => {
            return Err(ForbiddenLettersError::InvalidInput(
                "Invalid player word".to_string(),
            ))
        }
    };

    // Get a similar word that hasn't been used
    let mut attempts = 0;
    const MAX_ATTEMPTS: usize = 5;

    while attempts < MAX_ATTEMPTS {
        attempts += 1;

        // Try to find a similar word
        let next_word_result = get_similar_word(player_word, last_char, |x| {
            !used_words.contains(&x.to_string()) && !contains_forbidden_chars(x, forbidden_letters)
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
                    return Err(ForbiddenLettersError::Embedding(e));
                }
                // Try again
            }
        }
    }

    Err(ForbiddenLettersError::NoValidWords(format!(
        "Could not find a valid word without forbidden letters: {:?}",
        forbidden_letters
    )))
}

/// Provide a hint for the current turn
async fn provide_hint(
    bot: &Bot,
    chat_id: ChatId,
    curr_char: char,
    forbidden_letters: &[char],
) -> ResponseResult<()> {
    info!("Providing hint for chat {}", chat_id);

    // Get a random word starting with the current character without forbidden letters
    match get_random_word(
        |w| !contains_forbidden_chars(w, forbidden_letters),
        Some(curr_char),
    )
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
                    "I can't think of a hint right now. Just try any word starting with '{}' that doesn't contain {:?}.",
                    curr_char, forbidden_letters
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
    forbidden_letters: Vec<char>,
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
    match get_random_word(
        |w| {
            !contains_forbidden_chars(w, &forbidden_letters) && !used_stems.contains(&w.to_string())
        },
        Some(curr_char),
    )
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
                format!("Now your turn. Give a word starting with '{}'", next_char),
            )
            .await?;

            let _ = dialogue
                .update(ForbiddenLetters {
                    chain,
                    forbidden_letters,
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
async fn show_rules(bot: &Bot, chat_id: ChatId, forbidden_letters: &[char]) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        format!(
            "Forbidden Letters Rules:\n\
            1. Each word must start with the last letter of the previous word\n\
            2. No words may contain these forbidden letters: {:?}\n\
            3. No repeating words\n\
            4. Use /hint for a hint, /skip to skip your turn, or /stop to end the game",
            forbidden_letters
        ),
    )
    .await?;

    Ok(())
}

/// Check if a string contains any of the forbidden characters
fn contains_forbidden_chars(s: &str, forbidden_chars: &[char]) -> bool {
    for c in s.chars() {
        if forbidden_chars.contains(&c) {
            return true;
        }
    }
    false
}
