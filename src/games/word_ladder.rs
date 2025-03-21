use crate::state::MyDialogue;
use teloxide::prelude::{ChatId, ResponseResult};
use teloxide::Bot;

pub async fn start_word_ladder(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    // "word_ladder" => "Word Length Ladder! Climb up the word sizes.",
    todo!()
}
