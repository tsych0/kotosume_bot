use crate::state::MyDialogue;
use teloxide::prelude::{ChatId, ResponseResult};
use teloxide::Bot;

pub async fn start_synonym_string(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    // "synonym_string" => "Synonym String starts now! Link those meanings.",
    Ok(())
}
