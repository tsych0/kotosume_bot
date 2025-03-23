use crate::command::Command;
use crate::contains_any;
use crate::dictionary::{get_random_word, get_word_details, DictionaryError, WordInfo};
use crate::embeddings::{get_similar_word, similarity, EmbeddingError};
use crate::state::MyDialogue;
use crate::state::State::{Start, SynonymString};
use log::{error, info, warn};
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult};
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

/// Error type specific to Synonym String game
#[derive(Debug)]
enum SynonymError {
    Dictionary(DictionaryError),
    Embedding(EmbeddingError),
    InvalidInput(String),
    NoValidWords(String),
}

impl From<DictionaryError> for SynonymError {
    fn from(error: DictionaryError) -> Self {
        SynonymError::Dictionary(error)
    }
}

impl From<EmbeddingError> for SynonymError {
    fn from(error: EmbeddingError) -> Self {
        SynonymError::Embedding(error)
    }
}

impl std::fmt::Display for SynonymError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SynonymError::Dictionary(e) => write!(f, "Dictionary error: {}", e),
            SynonymError::Embedding(e) => write!(f, "Embedding error: {}", e),
            SynonymError::InvalidInput(msg) => write!(f, "{}", msg),
            SynonymError::NoValidWords(msg) => write!(f, "{}", msg),
        }
    }
}

/// Start a new Synonym String game
pub async fn start_synonym_string(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    info!("Starting Synonym String game for chat {}", chat_id);

    bot.send_message(chat_id, "Synonym String starts now! Link those meanings.")
        .await?;

    // Try to get a random word to start the game
    for _ in 0..3 {
        // Try up to 3 times
        match get_random_word(|_| true, None).await {
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

                info!("Synonym String started with word: {}", word.word);

                bot.send_message(chat_id, format!("First word: {}", word.word))
                    .await?;
                word.send_message(&bot, chat_id, 0).await?;

                bot.send_message(
                    chat_id,
                    format!(
                        "Now give a word starting with '{}' similar to {}",
                        curr_char, word.word
                    ),
                )
                .await?;

                let _ = dialogue
                    .update(SynonymString {
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

/// Handle player input during Synonym String game
pub async fn synonym_string(
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
                info!("Player stopped Synonym String game in chat {}", msg.chat.id);

                // Show final score/summary
                let player_words = chain.len() / 2;
                let bot_words = chain.len() - player_words;

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Game finished! Final score:\nYou: {} words\nBot: {} words\n\nSynonym chain: {}",
                        player_words,
                        bot_words,
                        chain.iter().map(|w| w.word.clone()).collect::<Vec<String>>().join(" → ")
                    ),
                ).await?;

                bot.send_message(
                    msg.chat.id,
                    "Synonym String game stopped. Thanks for playing!",
                )
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
    let prev_word = match chain.last() {
        Some(w) => &w.word,
        None => {
            error!("Chain is empty when processing player word");
            bot.send_message(chat_id, "Game error - please restart")
                .await?;
            let _ = dialogue.update(Start).await;
            return Ok(());
        }
    };

    // Check if word starts with the last letter of previous word
    // and is similar to the previous word
    if !word.starts_with(curr_char) {
        bot.send_message(
            chat_id,
            format!("Your word must start with '{}'", curr_char),
        )
        .await?;
        return Ok(());
    }

    let sim_score = similarity(&word, prev_word).unwrap_or(0.0);
    if sim_score < 0.8 {
        bot.send_message(
            chat_id,
            format!(
                "Your word '{}' is not similar enough to '{}'. Try something more related.",
                word, prev_word
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
            info!(
                "Player used word: {} in chat {} (similarity: {:.2})",
                word, chat_id, sim_score
            );
            let mut updated_stems = used_stems.clone();
            updated_stems.push(word.clone());

            word_details.send_message(&bot, chat_id, 0).await?;
            chain.push(word_details.clone());

            // Get the bot's response word
            match get_bot_response(&word, &updated_stems).await {
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
                            "Now give a word starting with '{}' similar to '{}'",
                            next_char, next_word_details.word
                        ),
                    )
                    .await?;

                    // Update game state
                    let _ = dialogue
                        .update(SynonymString {
                            chain,
                            curr_char: next_char,
                        })
                        .await;
                }
                Err(e) => {
                    error!("Failed to get bot response: {:?}", e);
                    bot.send_message(
                        chat_id,
                        "I can't think of a similar word! You win this round!",
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

/// Get the bot's response word with similarity constraints
async fn get_bot_response(
    player_word: &str,
    used_words: &[String],
) -> Result<WordInfo, SynonymError> {
    let mut used_words = used_words.to_vec();

    let last_char = match player_word.chars().last() {
        Some(c) => c,
        None => {
            return Err(SynonymError::InvalidInput(
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
            !used_words.contains(&x.to_string()) && similarity(player_word, x).unwrap_or(0.0) > 0.8
        });

        match next_word_result {
            Ok(word) => {
                // Try to get details for this word
                match get_word_details(&word).await {
                    Ok(details) => {
                        let sim_score = similarity(player_word, &word).unwrap_or(0.0);
                        info!(
                            "Bot found similar word '{}' (similarity: {:.2})",
                            word, sim_score
                        );

                        if contains_any(&used_words, &details.stems) {
                            used_words.extend(details.stems.clone());
                            continue;
                        }
                        return Ok(details);
                    }
                    Err(_) => {
                        used_words.push(word);
                        continue
                    }, // Try another word
                }
            }
            Err(e) => {
                if attempts == MAX_ATTEMPTS {
                    return Err(SynonymError::Embedding(e));
                }
                // Try again
            }
        }
    }

    Err(SynonymError::NoValidWords(format!(
        "Could not find a valid word similar to '{}'",
        player_word
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

    // Get a random word starting with the current character and similar to previous word
    match get_random_word(
        |w| similarity(w, prev_word).unwrap_or(0.0) > 0.8 && !used_stems.contains(&w.to_string()),
        Some(curr_char),
    )
    .await
    {
        Ok(hint) => {
            bot.send_message(
                chat_id,
                format!(
                    "Hint: You could try a word like '{}' or something similar to '{}'.",
                    hint.word, prev_word
                ),
            )
            .await?;
        }
        Err(_) => {
            bot.send_message(
                chat_id,
                format!(
                    "I can't think of a hint right now. Just try a word starting with '{}' that's similar to '{}'.",
                    curr_char, prev_word
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
    match get_random_word(
        |w| similarity(w, prev_word).unwrap_or(0.0) > 0.8 && !used_stems.contains(&w.to_string()),
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
                format!(
                    "Now your turn. Give a word starting with '{}' similar to '{}'",
                    next_char, word.word
                ),
            )
            .await?;

            let _ = dialogue
                .update(SynonymString {
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
async fn show_rules(bot: &Bot, chat_id: ChatId) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        "Synonym String Rules:\n\
        1. Each word must start with the last letter of the previous word\n\
        2. Each word must be similar in meaning to the previous word\n\
        3. No repeating words\n\
        4. Use /hint for a hint, /skip to skip your turn, or /stop to end the game",
    )
    .await?;

    Ok(())
}
