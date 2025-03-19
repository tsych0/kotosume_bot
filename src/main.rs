mod command;
mod dictionary;
mod embeddings;
mod games;
mod handler;
mod state;

use std::error::Error;
use teloxide::types::{MaybeInaccessibleMessage, Me};
use teloxide::utils::command::BotCommands;
use teloxide::{prelude::*, types::InlineKeyboardButton, types::InlineKeyboardMarkup};

// Main bot setup with both message and callback handlers
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting word game bot ...");

    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(handler::message_handler))
        .branch(Update::filter_callback_query().endpoint(handler::callback_handler));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
