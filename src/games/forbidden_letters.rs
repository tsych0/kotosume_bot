use crate::state::MyDialogue;
use teloxide::prelude::{ChatId, ResponseResult};
use teloxide::Bot;

pub async fn start_forbidden_letters(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    // "forbidden_letters" => "Forbidden Letters! Avoid the banned ones.",
    Ok(())
}
