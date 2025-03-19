use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::Dialogue;
use crate::dictionary::WordInfo;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    WordChain(Vec<WordInfo>),
    AlphabetSprint(Vec<WordInfo>),
    RhymeTime(Vec<WordInfo>),
    LastLetterScramble(Vec<WordInfo>),
    SynonymString(Vec<WordInfo>),
    WordLengthLadder(Vec<WordInfo>),
    ForbiddenLetters(Vec<WordInfo>),
}
