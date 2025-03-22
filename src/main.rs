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
use std::collections::HashSet;
use std::error::Error;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use tokio::signal;

// Main bot setup with both message and callback handlers
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_cache().await;
    dotenv::dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting word game bot ...");

    let bot = Bot::from_env();

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

    tokio::spawn(async move {
        signal::ctrl_c()
            .await
            .expect("Failed to listen for shutdown signal");
        save_cache(&get_cache(), "cache.bin").expect("Failed to save cache");
        println!("Cache saved before shutdown.");
    });

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

pub fn contains_any(vec1: &[String], vec2: &[String]) -> bool {
    let set: HashSet<_> = vec1.iter().collect();
    vec2.iter().any(|s| set.contains(s))
}
