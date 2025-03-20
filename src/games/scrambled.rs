use crate::state::MyDialogue;
use teloxide::prelude::{ChatId, ResponseResult};
use teloxide::Bot;

pub async fn start_last_letter_scramble(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    // "last_letter" => "Last Letter Scramble! Let’s twist those endings.",
    Ok(())
}
