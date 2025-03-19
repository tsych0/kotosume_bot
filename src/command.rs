use teloxide::macros::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Wordplay Bot Commands")]
pub enum Command {
    #[command(description = "Start the bot and show the game menu")]
    Start,
    #[command(description = "Play a random game")]
    Play,
    #[command(description = "Get a hint for the current game")]
    Hint,
    #[command(description = "Skip the current game")]
    Skip,
    #[command(description = "Check your score")]
    Score,
    #[command(description = "Show rules for the current game")]
    Rules,
    #[command(description = "View your stats")]
    Stats,
    #[command(description = "Stop the current game")]
    Stop,
}
