use crate::state::MyDialogue;
use teloxide::prelude::{ChatId, ResponseResult};
use teloxide::Bot;

pub async fn start_rhyme_time(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    // "rhyme_time" => "Rhyme Time begins! Get those rhymes flowing.",
    Ok(())
}
