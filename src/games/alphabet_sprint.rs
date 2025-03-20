use crate::state::MyDialogue;
use teloxide::prelude::{ChatId, ResponseResult};
use teloxide::Bot;

pub async fn start_alphabet_sprint(
    chat_id: ChatId,
    bot: Bot,
    dialogue: MyDialogue,
) -> ResponseResult<()> {
    Ok(())
    // "alphabet_sprint" => "Alphabet Sprint time! Ready to race through the letters?",
}
