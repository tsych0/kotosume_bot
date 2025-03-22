use crate::dictionary::WordInfo;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::Dialogue;

pub type MyDialogue = Dialogue<State, InMemStorage<State>>;
// pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    WordChain {
        chain: Vec<WordInfo>,
        curr_char: char,
    },
    AlphabetSprint {
        alphabet: char,
        words: Vec<WordInfo>,
    },
    LastLetterScramble {
        level: u8,
        chain: Vec<WordInfo>,
        curr_char: char,
    },
    SynonymString {
        chain: Vec<WordInfo>,
        curr_char: char,
    },
    WordLengthLadder {
        curr_len: u8,
        max_len: u8,
        chain: Vec<WordInfo>,
        curr_char: char,
    },
    ForbiddenLetters {
        forbidden_letters: Vec<char>,
        chain: Vec<WordInfo>,
        curr_char: char,
    },
}
