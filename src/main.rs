mod command;
mod dictionary;
mod embeddings;
mod games;
mod handler;
mod state;

use crate::dictionary::{get_cache, init_cache, save_cache};
use crate::games::alphabet_sprint::alphabet_sprint;
use crate::games::forbidden_letters::forbidden_letters;
use crate::games::scrambled::last_letter_scramble;
use crate::games::synonym_string::synonym_string;
use crate::games::word_chain::word_chain;
use crate::games::word_ladder::word_ladder;
use crate::state::State;
use log::{error, info};
use std::collections::HashSet;
use std::error::Error;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use tokio::signal;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Initialize environment and logging
fn initialize_environment() -> Result<()> {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    info!("Environment initialized");
    Ok(())
}

/// Initialize the bot's cache
async fn initialize_cache() -> Result<()> {
    info!("Initializing cache...");
    init_cache().await;
    info!("Cache initialized");
    Ok(())
}

/// Create and configure the bot's dispatcher
fn create_dispatcher(
    bot: Bot,
) -> Dispatcher<Bot, teloxide::RequestError, teloxide::dispatching::DefaultKey> {
    info!("Creating dispatcher...");

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .enter_dialogue::<Message, InMemStorage<State>, State>()
                .branch(dptree::case![State::Start].endpoint(handler::message_handler))
                .branch(dptree::case![State::WordChain { chain, curr_char }].endpoint(word_chain))
                .branch(
                    dptree::case![State::ForbiddenLetters {
                        forbidden_letters,
                        chain,
                        curr_char
                    }]
                    .endpoint(forbidden_letters),
                )
                .branch(
                    dptree::case![State::AlphabetSprint { alphabet, words }]
                        .endpoint(alphabet_sprint),
                )
                .branch(
                    dptree::case![State::LastLetterScramble {
                        level,
                        chain,
                        curr_char
                    }]
                    .endpoint(last_letter_scramble),
                )
                .branch(
                    dptree::case![State::WordLengthLadder {
                        curr_len,
                        max_len,
                        chain,
                        curr_char
                    }]
                    .endpoint(word_ladder),
                )
                .branch(
                    dptree::case![State::SynonymString { chain, curr_char }]
                        .endpoint(synonym_string),
                ),
        )
        .branch(
            Update::filter_callback_query()
                .enter_dialogue::<CallbackQuery, InMemStorage<State>, State>()
                .endpoint(handler::callback_handler),
        );

    info!("Dispatcher created");

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
}

/// Setup signal handler for graceful shutdown
fn setup_shutdown_handler() -> Result<()> {
    info!("Setting up shutdown handler...");

    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(_) => {
                info!("Shutdown signal received, saving cache...");
                match save_cache(&get_cache(), "cache.bin") {
                    Ok(_) => info!("Cache saved successfully before shutdown"),
                    Err(e) => error!("Failed to save cache: {}", e),
                }
            }
            Err(e) => error!("Failed to listen for shutdown signal: {}", e),
        }
    });

    Ok(())
}

// Main bot setup with both message and callback handlers
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment and components
    initialize_environment()?;
    initialize_cache().await?;
    info!("Starting word game bot...");

    // Create the bot instance
    let bot = Bot::from_env();

    // Setup graceful shutdown handler
    setup_shutdown_handler()?;

    // Create and run the dispatcher
    let mut dispatcher = create_dispatcher(bot);

    // Start the bot and wait for it to finish
    info!("Bot is now running!");
    dispatcher.dispatch().await;

    Ok(())
}

/// Utility function to check if any items from the first vector exist in the second vector
pub fn contains_any(vec1: &[String], vec2: &[String]) -> bool {
    let set: HashSet<_> = vec1.iter().collect();
    vec2.iter().any(|s| set.contains(s))
}
