# Wordplay Bot

A Telegram chatbot that brings fun and challenging word games to your fingertips! Test your vocabulary, speed, and creativity with a variety of linguistic puzzles.

## About

Wordplay Bot offers a collection of brain-teasing word games for language lovers. Whether you're a casual player or a word nerd, dive in and enjoy the twists and turns of wordplay!

## Features

- Interactive game menu to choose your challenge.
- Commands: `/start`, `/play`, `/hint`, `/skip`, `/score`, `/rules`, `/stats`, `/stop`.
- Modular game structure for easy expansion.
- Built with Rust and Teloxide for a fast, reliable experience.

## Games

Expect a mix of clever word-based challenges that keep you entertained and sharp. (More details in-game!)

## Project Structure

- `src/main.rs`: Entry point for the bot.
- `src/command.rs`: Defines bot commands.
- `src/dictionary.rs`: Manages word validation (placeholder).
- `src/embeddings.rs`: Handles word embeddings for synonym games (placeholder).
- `src/handler.rs`: Processes incoming messages and callbacks.
- `src/state.rs`: Manages game state (placeholder).
- `src/games/`: Contains individual game modules:
   - `alphabet_sprint.rs`
   - `forbidden_letters.rs`
   - `rhyme_time.rs`
   - `scramble.rs`
   - `word_chain.rs`
   - `word_ladder.rs`

## Installation

1. **Prerequisites**:
   - Rust (latest stable version)
   - A Telegram Bot Token from [BotFather](https://t.me/BotFather)

2. **Clone the Repository**:
   ```bash
   git clone https://github.com/veryshyjelly/kotosume_bot.git
   cd kotosume_bot
   ```

3. **Install Dependencies**:
   ```bash
   cargo build
   ```

4. **Set Environment Variable**:
   ```bash
   export TELOXIDE_TOKEN="your_bot_token_here"
   ```

5. **Run the Bot**:
   ```bash
   cargo run
   ```

## Usage

- Start the bot with `/start` to see the game menu.
- Select a game via the inline keyboard.
- Use commands like `/hint` or `/skip` during gameplay.

## Development

- **Tech Stack**: Rust, Teloxide, Tokio
- **Adding a New Game**:
   1. Create a new file in `src/games/` (e.g., `new_game.rs`).
   2. Add the module to `src/games/mod.rs`.
   3. Update the `get_game_menu` function in `src/games/mod.rs`.
   4. Implement game logic in the new module.
- **Contributing**: Feel free to fork, tweak, and submit PRs!

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contact

Built with ❤️ by [@veryshyjelly](github.com/veryshyjelly). Reach out via Telegram: [@veryshyjelly](https://t.me/veryshyhjelly).
