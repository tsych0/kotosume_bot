use crate::dictionary::WordInfo;
use std::fmt;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::Dialogue;

/// Type alias for dialogues with our state machine
pub type MyDialogue = Dialogue<State, InMemStorage<State>>;
// pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Game state machine representing different game modes and their state
#[derive(Clone, Default, Debug)]
pub enum State {
    /// Initial state, no active game
    #[default]
    Start,

    /// Word Chain game: players continue a chain where each word starts with the last letter of the previous
    WordChain {
        /// List of words in the current chain
        chain: Vec<WordInfo>,
        /// Current character that the next word must start with
        curr_char: char,
    },

    /// Alphabet Sprint: players provide words starting with a specific letter
    AlphabetSprint {
        /// Current alphabet letter to use
        alphabet: char,
        /// Words already provided for the current letter
        words: Vec<WordInfo>,
    },

    /// Last Letter Scramble: words must start with last letter of previous word plus scrambling rules
    LastLetterScramble {
        /// Difficulty level (higher means more scrambling)
        level: u8,
        /// List of words in the current chain
        chain: Vec<WordInfo>,
        /// Current character that the next word must start with
        curr_char: char,
    },

    /// Synonym String: words must be synonyms or related to the previous word
    SynonymString {
        /// List of words in the current chain
        chain: Vec<WordInfo>,
        /// Current character that the next word must start with
        curr_char: char,
    },

    /// Word Length Ladder: words increase or decrease in length progressively
    WordLengthLadder {
        /// Current word length requirement
        curr_len: u8,
        /// Maximum word length in this ladder
        max_len: u8,
        /// List of words in the current chain
        chain: Vec<WordInfo>,
        /// Current character that the next word must start with
        curr_char: char,
    },

    /// Forbidden Letters: words must not contain certain letters
    ForbiddenLetters {
        /// Letters that cannot be used in words
        forbidden_letters: Vec<char>,
        /// List of words in the current chain
        chain: Vec<WordInfo>,
        /// Current character that the next word must start with
        curr_char: char,
    },
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Start => write!(f, "No active game"),
            State::WordChain { curr_char, chain } => {
                write!(
                    f,
                    "Word Chain - Next letter: '{}', Chain length: {}",
                    curr_char,
                    chain.len()
                )
            }
            State::AlphabetSprint { alphabet, words } => {
                write!(
                    f,
                    "Alphabet Sprint - Current letter: '{}', Words: {}",
                    alphabet,
                    words.len()
                )
            }
            State::LastLetterScramble {
                level,
                curr_char,
                chain,
            } => {
                write!(
                    f,
                    "Last Letter Scramble - Level: {}, Next letter: '{}', Chain length: {}",
                    level,
                    curr_char,
                    chain.len()
                )
            }
            State::SynonymString { curr_char, chain } => {
                write!(
                    f,
                    "Synonym String - Next letter: '{}', Chain length: {}",
                    curr_char,
                    chain.len()
                )
            }
            State::WordLengthLadder {
                curr_len,
                max_len,
                curr_char,
                chain,
            } => {
                write!(f, "Word Length Ladder - Current length: {}, Max length: {}, Next letter: '{}', Chain length: {}", 
                       curr_len, max_len, curr_char, chain.len())
            }
            State::ForbiddenLetters {
                forbidden_letters,
                curr_char,
                chain,
            } => {
                write!(
                    f,
                    "Forbidden Letters - Forbidden: '{}', Next letter: '{}', Chain length: {}",
                    forbidden_letters.iter().collect::<String>(),
                    curr_char,
                    chain.len()
                )
            }
        }
    }
}
