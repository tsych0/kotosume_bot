mod command;
mod dictionary;
mod embeddings;
mod games;
mod handler;
mod state;

use crate::dictionary::{get_cache, init_cache, save_cache};
use crate::games::word_chain::word_chain;
use crate::state::State;
use std::error::Error;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::types::{MaybeInaccessibleMessage, Me};
use teloxide::utils::command::BotCommands;
use teloxide::{prelude::*, types::InlineKeyboardButton, types::InlineKeyboardMarkup};
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
                .branch(dptree::case![State::WordChain { chain }].endpoint(word_chain)),
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
