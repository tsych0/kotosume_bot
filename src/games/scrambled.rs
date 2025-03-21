use crate::dictionary::{get_random_word, WordInfo};
use crate::state::MyDialogue;
use crate::state::State::LastLetterScramble;
use std::collections::HashSet;
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult};
use teloxide::types::Me;
use teloxide::Bot;

pub async fn start_last_letter_scramble(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    bot.send_message(chat_id, "Last Letter Scramble! Let’s twist those endings.")
        .await?;

    loop {
        if let Ok(word) = get_random_word(|_| true).await {
            bot.send_message(chat_id, format!("First word: {}", word.word))
                .await?;
            word.send_message(&bot, chat_id, 0).await?;
            bot.send_message(
                chat_id,
                format!(
                    "Now give a word starting with '{}'",
                    word.word.chars().last().unwrap()
                ),
            )
            .await?;
            let _ = dialogue
                .update(LastLetterScramble {
                    chain: vec![word],
                    level: 2,
                })
                .await;
            return Ok(());
        }
    }
}

pub async fn last_letter_scramble(
    bot: Bot,
    dialogue: MyDialogue,
    (chain, level): (Vec<WordInfo>, u8),
    msg: Message,
    me: Me,
) -> ResponseResult<()> {
    todo!()
}

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
