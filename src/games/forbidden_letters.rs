use rand::prelude::IteratorRandom;
use rand::rng;
use crate::state::MyDialogue;
use teloxide::prelude::{ChatId, Requester, ResponseResult};
use teloxide::Bot;
use crate::dictionary::get_random_word;
use crate::state::State::ForbiddenLetters;

pub async fn start_forbidden_letters(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        "Forbidden Letters! Avoid the banned ones.",
    )
        .await?;

    let forbidden_letters = ('a'..'z').choose_multiple(&mut rng(), 1);

    loop {
        if let Ok(word) = get_random_word().await {
            bot.send_message(chat_id, format!("Forbidden Letters! Avoid {:?}", forbidden_letters)).await?;
            bot.send_message(chat_id, format!("First word: {}", word.word))
                .await?;
            bot.send_message(chat_id, word.to_string()).await?;
            bot.send_message(
                chat_id,
                format!(
                    "Now give a word starting with '{}'",
                    word.word.chars().last().unwrap()
                ),
            )
                .await?;

            let _ = dialogue.update(ForbiddenLetters { chain: vec![word], forbidden_letters }).await;
            return Ok(());
        }
    }
}
