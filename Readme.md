# Kotosume Bot

A Telegram chatbot that brings fun and challenging word games to your fingertips! Test your vocabulary, speed, and creativity with a variety of linguistic puzzles designed to enhance your language skills.

## About

Kotosume Bot is a word games platform that combines entertainment with language learning. The bot offers multiple engaging word games that challenge users to think creatively with words, synonyms, and language patterns. It's perfect for casual players, word enthusiasts, and language learners alike.

## Features

- **Multiple Word Games**: Choose from six different word-based challenges
- **Interactive Commands**: `/start`, `/play`, `/hint`, `/skip`, `/score`, `/rules`, `/stats`, `/stop`
- **Dictionary Integration**: Real word validation and definitions
- **Intelligent Responses**: Bot suggests and responds with semantically appropriate words
- **Game State Management**: Resume games where you left off
- **Score Tracking**: Keep track of your performance in each game
- **Helpful Hints**: Get assistance when stuck

## Games

### Word Chain
Link words where each new word must start with the last letter of the previous word. Build the longest chain possible!

### Word Ladder
Start with short words and gradually increase word length with each turn. Challenge yourself to reach longer and more complex words.

### Last Letter Scramble
Similar to Word Chain, but with varying difficulty levels that require words to contain specific letter patterns.

### Synonym String
Create a chain of words with similar meanings, where each word starts with the last letter of the previous word.

### Alphabet Sprint
Race through words that all start with the same letter. How many words can you find?

### Forbidden Letters
Play word chain while avoiding words that contain certain forbidden letters. A true vocabulary challenge!

## Project Structure

- `src/main.rs`: Entry point and dispatcher configuration
- `src/command.rs`: Bot command definitions and parsing
- `src/dictionary.rs`: Word validation, retrieval, and definition lookup
- `src/embeddings.rs`: Word embedding operations for finding similar words
- `src/state.rs`: Game state management and persistence
- `src/games/`: Individual game modules:
   - `word_chain.rs`: Classic word chain game implementation
   - `word_ladder.rs`: Word length ladder game
   - `scrambled.rs`: Last letter scramble with difficulty levels
   - `synonym_string.rs`: Chain of synonymous words
   - `alphabet_sprint.rs`: Words starting with the same letter
   - `forbidden_letters.rs`: Word chain avoiding certain letters

## Technical Implementation

The bot is built with:

- **Rust**: For performance, safety, and reliability
- **Teloxide**: Telegram bot framework for Rust
- **Tokio**: Asynchronous runtime for handling concurrent operations
- **Word Embeddings**: Vector representations of words to find semantically similar terms
- **Functional Programming**: Extensive use of Rust's functional paradigms
- **State Management**: Dialogue-based state tracking for multiple concurrent games

## Installation

1. **Prerequisites**:
   - Rust (latest stable version)
   - A Telegram Bot Token from [BotFather](https://t.me/BotFather)
   - Word embeddings database (see documentation)

2. **Clone the Repository**:
   ```bash
   git clone https://github.com/yourusername/kotosume_bot.git
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

1. Start a chat with the bot on Telegram
2. Use `/start` to see the welcome message and game menu
3. Use `/play` to select a game from the menu
4. Follow the game instructions and use commands as needed:
   - `/hint`: Get a suggestion when stuck
   - `/skip`: Skip your turn
   - `/score`: See current game score
   - `/rules`: Review game rules
   - `/stop`: End the current game

## Development

### Architecture

The bot follows a modular architecture:
- **Main Dispatcher**: Routes commands and messages to appropriate handlers
- **Game Modules**: Self-contained game logic with common interfaces
- **Dictionary Service**: Handles word validation and information retrieval
- **Embeddings Service**: Provides semantic word relationships
- **State Management**: Tracks game progress and user interactions

### Adding a New Game

1. Create a new file in `src/games/` (e.g., `new_game.rs`)
2. Implement the game module with:
   - Start function to initialize the game
   - Main game handler function
   - Helper functions for game logic
   - Custom error type and handlers
3. Add the module to `src/games/mod.rs`
4. Update the game selection menu

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contact

For questions, suggestions or contributions, please open an issue on the GitHub repository or reach out to the maintainers directly.
