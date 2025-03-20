use crate::state::MyDialogue;
use teloxide::prelude::{ChatId, Requester, ResponseResult};
use teloxide::Bot;
use crate::dictionary::get_random_word;
use crate::state::State::{AlphabetSprint, WordChain};

pub async fn start_alphabet_sprint(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        "Alphabet Sprint time! Ready to race through the letters?",
    )
        .await?;

    loop {
        if let Ok(word) = get_random_word().await {
            bot.send_message(chat_id, format!("First word: {}", word.word))
                .await?;
            bot.send_message(chat_id, word.to_string()).await?;
            bot.send_message(
                chat_id,
                format!(
                    "Now give a word starting with '{}'",
                    word.word.chars().next().unwrap()
                ),
            )
                .await?;
            let _ = dialogue.update(AlphabetSprint { words: vec![word], alphabet: word.word.chars().next().unwrap() }).await;
            return Ok(());
        }
    }
}
