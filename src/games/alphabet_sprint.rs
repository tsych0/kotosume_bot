use crate::command::Command;
use crate::contains_any;
use crate::dictionary::{get_random_word, get_word_details, DictionaryError, WordInfo};
use crate::embeddings::{get_similar_word, EmbeddingError};
use crate::state::MyDialogue;
use crate::state::State::{AlphabetSprint, Start};
use log::{error, info, warn};
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult};
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

/// Error type specific to Alphabet Sprint game
#[derive(Debug)]
enum AlphabetSprintError {
    Dictionary(DictionaryError),
    Embedding(EmbeddingError),
    InvalidInput(String),
    NoValidWords(String),
}

impl From<DictionaryError> for AlphabetSprintError {
    fn from(error: DictionaryError) -> Self {
        AlphabetSprintError::Dictionary(error)
    }
}

impl From<EmbeddingError> for AlphabetSprintError {
    fn from(error: EmbeddingError) -> Self {
        AlphabetSprintError::Embedding(error)
    }
}

impl std::fmt::Display for AlphabetSprintError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AlphabetSprintError::Dictionary(e) => write!(f, "Dictionary error: {}", e),
            AlphabetSprintError::Embedding(e) => write!(f, "Embedding error: {}", e),
            AlphabetSprintError::InvalidInput(msg) => write!(f, "{}", msg),
            AlphabetSprintError::NoValidWords(msg) => write!(f, "{}", msg),
        }
    }
}

/// Start a new Alphabet Sprint game
pub async fn start_alphabet_sprint(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    info!("Starting Alphabet Sprint game for chat {}", chat_id);

    bot.send_message(
        chat_id,
        "Alphabet Sprint time! Ready to race through the letters?",
    )
    .await?;

    // Try to get a random word to start the game
    for _ in 0..3 {
        // Try up to 3 times
        match get_random_word(|_| true, None).await {
            Ok(word) => {
                let start_char = match word.word.chars().next() {
                    Some(c) => c,
                    None => {
                        error!("Selected word '{}' has no characters", word.word);
                        bot.send_message(chat_id, "Error starting game, please try again.")
                            .await?;
                        return Ok(());
                    }
                };

                info!("Alphabet Sprint started with letter: {}", start_char);

                bot.send_message(chat_id, format!("First word: {}", word.word))
                    .await?;
                word.send_message(&bot, chat_id, 0).await?;

                bot.send_message(
                    chat_id,
                    format!("Now give a word starting with '{}'", start_char),
                )
                .await?;

                let _ = dialogue
                    .update(AlphabetSprint {
                        words: vec![word.clone()],
                        alphabet: start_char,
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

/// Handle player input during Alphabet Sprint game
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
                bot.send_message(
                    msg.chat.id,
                    "Please stop this game first with /stop to use this command.",
                )
                .await?;
            }
            Ok(Command::Hint) => {
                provide_hint(&bot, msg.chat.id, alphabet, &words).await?;
            }
            Ok(Command::Skip) => {
                skip_turn(&bot, msg.chat.id, dialogue, words, alphabet).await?;
            }
            Ok(Command::Score) => {
                show_score(&bot, msg.chat.id, &words).await?;
            }
            Ok(Command::Rules) => {
                show_rules(&bot, msg.chat.id).await?;
            }
            Ok(Command::Stop) => {
                info!(
                    "Player stopped Alphabet Sprint game in chat {}",
                    msg.chat.id
                );

                // Show final score
                let player_words = words.len() / 2;
                let bot_words = words.len() - player_words;

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Game finished! Final score:\nYou: {} words\nBot: {} words\n\nWords played: {}",
                        player_words,
                        bot_words,
                        words.iter().map(|w| w.word.clone()).collect::<Vec<String>>().join(", ")
                    ),
                ).await?;

                bot.send_message(
                    msg.chat.id,
                    "Alphabet Sprint game stopped. Thanks for playing!",
                )
                .await?;
                let _ = dialogue.update(Start).await;
            }
            Err(_) => {
                process_player_word(text, bot, dialogue, words, alphabet, msg.chat.id).await?;
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
    alphabet: char,
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

    // Check if word starts with the current alphabet
    if !word.starts_with(alphabet) {
        bot.send_message(chat_id, format!("Your word must start with '{}'", alphabet))
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
            match get_bot_response(&word, &updated_stems, alphabet).await {
                Ok(next_word_details) => {
                    chain.push(next_word_details.clone());
                    bot.send_message(chat_id, format!("My word: {}", next_word_details.word))
                        .await?;
                    next_word_details.send_message(&bot, chat_id, 0).await?;

                    // Prompt for the next word
                    bot.send_message(
                        chat_id,
                        format!(
                            "Now your turn. Give another word starting with '{}'",
                            alphabet
                        ),
                    )
                    .await?;

                    // Update game state
                    let _ = dialogue
                        .update(AlphabetSprint {
                            alphabet,
                            words: chain,
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

/// Get the bot's response word starting with the same alphabet
async fn get_bot_response(
    player_word: &str,
    used_words: &[String],
    alphabet: char,
) -> Result<WordInfo, AlphabetSprintError> {
    // Get a similar word that hasn't been used
    let mut attempts = 0;
    const MAX_ATTEMPTS: usize = 3;

    while attempts < MAX_ATTEMPTS {
        attempts += 1;

        // Try to find a similar word
        let next_word_result = get_similar_word(player_word, alphabet, |x| {
            !used_words.contains(&x.to_string())
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
                    return Err(AlphabetSprintError::Embedding(e));
                }
                // Try again
            }
        }
    }

    Err(AlphabetSprintError::NoValidWords(format!(
        "Could not find a valid word starting with '{}'",
        alphabet
    )))
}

/// Provide a hint for the current turn
async fn provide_hint(
    bot: &Bot,
    chat_id: ChatId,
    alphabet: char,
    words: &[WordInfo],
) -> ResponseResult<()> {
    info!("Providing hint for chat {}", chat_id);

    let used_stems = words
        .iter()
        .flat_map(|x| x.stems.clone())
        .collect::<Vec<String>>();

    // Get a random word starting with the current alphabet (not used before)
    match get_random_word(|w| !used_stems.contains(&w.to_string()), Some(alphabet)).await {
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
                    alphabet
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
    mut words: Vec<WordInfo>,
    alphabet: char,
) -> ResponseResult<()> {
    info!("Player skipped turn in chat {}", chat_id);

    bot.send_message(chat_id, "Skipping your turn...").await?;

    // Get list of used words
    let used_stems = words
        .iter()
        .flat_map(|x| x.stems.clone())
        .collect::<Vec<String>>();

    // Try to get a word for the bot
    match get_random_word(|w| !used_stems.contains(&w.to_string()), Some(alphabet)).await {
        Ok(word) => {
            bot.send_message(chat_id, format!("My word: {}", word.word))
                .await?;
            word.send_message(bot, chat_id, 0).await?;
            words.push(word.clone());

            bot.send_message(
                chat_id,
                format!("Now your turn. Give a word starting with '{}'", alphabet),
            )
            .await?;

            let _ = dialogue.update(AlphabetSprint { alphabet, words }).await;
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
async fn show_score(bot: &Bot, chat_id: ChatId, words: &[WordInfo]) -> ResponseResult<()> {
    let player_words = words.len() / 2;
    let bot_words = words.len() - player_words;

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
        "Alphabet Sprint Rules:\n\
        1. We'll focus on words starting with the same letter\n\
        2. Take turns giving words that start with that letter\n\
        3. No repeating words\n\
        4. Use /hint for a hint, /skip to skip your turn, or /stop to end the game",
    )
    .await?;

    Ok(())
}
