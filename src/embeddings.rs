use itertools::Itertools;
use log::{info, warn};
use std::collections::HashMap;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;
use std::sync::OnceLock;

/// Error type for embedding operations
#[derive(Debug)]
pub enum EmbeddingError {
    IoError(io::Error),
    ParseError(String),
    MissingData(String),
    InvalidWord(String),
}

impl From<io::Error> for EmbeddingError {
    fn from(error: io::Error) -> Self {
        EmbeddingError::IoError(error)
    }
}

impl std::fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EmbeddingError::IoError(e) => write!(f, "I/O error: {}", e),
            EmbeddingError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            EmbeddingError::MissingData(msg) => write!(f, "Missing data: {}", msg),
            EmbeddingError::InvalidWord(word) => write!(f, "Invalid word: {}", word),
        }
    }
}

/// Type alias for embedding vectors
pub type EmbeddingVec = Vec<f64>;
/// Type alias for word-to-embedding maps grouped by first letter
pub type EmbeddingMap = HashMap<char, HashMap<String, EmbeddingVec>>;

static EMBEDDINGS: OnceLock<EmbeddingMap> = OnceLock::new();
const EMBEDDINGS_FILE: &str = "word2vec.txt";

/// Initialize embeddings from a file
fn init(file_name: &str) -> Result<EmbeddingMap, EmbeddingError> {
    info!("Initializing embeddings from {}", file_name);

    if !Path::new(file_name).exists() {
        return Err(EmbeddingError::IoError(io::Error::new(
            ErrorKind::NotFound,
            format!("Embeddings file not found: {}", file_name),
        )));
    }

    let content = fs::read_to_string(file_name)?;
    info!("Loaded embeddings file, processing {} bytes", content.len());

    let result: EmbeddingMap = content
        .replace("\r", "")
        .lines()
        .filter_map(|line| {
            let mut word_iter = line.split_whitespace();
            let word = match word_iter.next() {
                Some(w) => w.to_string(),
                None => {
                    warn!("Empty line in embeddings file");
                    return None;
                }
            };

            let vec: Result<Vec<f64>, _> = word_iter
                .map(|x| x.parse::<f64>().map_err(|e| e.to_string()))
                .collect();

            match vec {
                Ok(v) => Some((word, v)),
                Err(e) => {
                    warn!("Failed to parse embedding for word '{}': {}", word, e);
                    None
                }
            }
        })
        .into_iter()
        .sorted_by_key(|(key, _)| {
            key.chars().next().unwrap_or('_') // Default to underscore for empty words
        })
        .chunk_by(|(key, _)| key.chars().next().unwrap_or('_'))
        .into_iter()
        .map(|(first_char, group)| {
            let inner_map = group.into_iter().collect::<HashMap<_, _>>();
            (first_char, inner_map)
        })
        .collect();

    info!(
        "Embeddings initialized with {} first characters",
        result.len()
    );
    Ok(result)
}

/// Get the global embeddings map, initializing if necessary
pub fn get_embeddings() -> Result<&'static EmbeddingMap, EmbeddingError> {
    match EMBEDDINGS.get() {
        Some(embeddings) => Ok(embeddings),
        None => {
            let embeddings = init(EMBEDDINGS_FILE)?;
            EMBEDDINGS
                .set(embeddings)
                .expect("Failed to set embeddings");
            Ok(EMBEDDINGS.get().unwrap())
        }
    }
}

/// Check if a word exists in the embeddings
pub fn is_valid_word(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }

    let first_char = match word.chars().next() {
        Some(c) => c,
        None => return false,
    };

    match get_embeddings() {
        Ok(embeddings) => embeddings
            .get(&first_char)
            .map_or(false, |map| map.contains_key(word)),
        Err(_) => false,
    }
}

/// Find the most similar word to the given word that starts with the specified character
/// and satisfies the predicate
pub fn get_similar_word<P>(
    word: &str,
    starting_char: char,
    predicate: P,
) -> Result<String, EmbeddingError>
where
    P: Fn(&str) -> bool,
{
    let embeddings = get_embeddings()?;

    // Validate input word
    if word.is_empty() {
        return Err(EmbeddingError::InvalidWord("Word is empty".to_string()));
    }

    let first_char = word
        .chars()
        .next()
        .ok_or_else(|| EmbeddingError::InvalidWord("Word is empty".to_string()))?;

    let f_map = embeddings.get(&first_char).ok_or_else(|| {
        EmbeddingError::MissingData(format!("No embeddings for letter '{}'", first_char))
    })?;

    if !f_map.contains_key(word) {
        return Err(EmbeddingError::InvalidWord(format!(
            "Word '{}' not found in embeddings",
            word
        )));
    }

    // Get map for target starting character
    let s_map = embeddings.get(&starting_char).ok_or_else(|| {
        EmbeddingError::MissingData(format!("No embeddings for letter '{}'", starting_char))
    })?;

    // Find the most similar word
    let result = s_map
        .keys()
        .filter(|x| predicate(x))
        .collect::<Vec<&String>>();

    if result.is_empty() {
        return Err(EmbeddingError::MissingData(format!(
            "No words starting with '{}' match the predicate",
            starting_char
        )));
    }

    // Find the most similar word to the input word
    let mut best_similarity = -1.0;
    let mut best_word = String::new();

    for candidate in result {
        match similarity_eff(word, f_map, candidate, s_map) {
            Ok(sim) => {
                if sim > best_similarity {
                    best_similarity = sim;
                    best_word = candidate.clone();
                }
            }
            Err(_) => continue, // Skip words with errors
        }
    }

    if best_word.is_empty() {
        return Err(EmbeddingError::MissingData(format!(
            "Could not find a similar word starting with '{}'",
            starting_char
        )));
    }

    Ok(best_word)
}

/// Calculate similarity between two words
pub fn similarity(a: &str, b: &str) -> Result<f64, EmbeddingError> {
    if a.is_empty() || b.is_empty() {
        return Err(EmbeddingError::InvalidWord(
            "Words cannot be empty".to_string(),
        ));
    }

    let embeddings = get_embeddings()?;
    let a_first = a
        .chars()
        .next()
        .ok_or_else(|| EmbeddingError::InvalidWord("Word is empty".to_string()))?;
    let b_first = b
        .chars()
        .next()
        .ok_or_else(|| EmbeddingError::InvalidWord("Word is empty".to_string()))?;

    let a_embed_map = embeddings.get(&a_first).ok_or_else(|| {
        EmbeddingError::MissingData(format!("No embeddings for letter '{}'", a_first))
    })?;
    let b_embed_map = embeddings.get(&b_first).ok_or_else(|| {
        EmbeddingError::MissingData(format!("No embeddings for letter '{}'", b_first))
    })?;

    let a_embed = a_embed_map.get(a).ok_or_else(|| {
        EmbeddingError::InvalidWord(format!("Word '{}' not found in embeddings", a))
    })?;
    let b_embed = b_embed_map.get(b).ok_or_else(|| {
        EmbeddingError::InvalidWord(format!("Word '{}' not found in embeddings", b))
    })?;

    Ok(cosine(a_embed, b_embed))
}

/// Helper function to calculate similarity efficiently when maps are already available
fn similarity_eff(
    a: &str,
    a_embed_map: &HashMap<String, Vec<f64>>,
    b: &str,
    b_embed_map: &HashMap<String, Vec<f64>>,
) -> Result<f64, EmbeddingError> {
    let a_embed = a_embed_map.get(a).ok_or_else(|| {
        EmbeddingError::InvalidWord(format!("Word '{}' not found in embeddings", a))
    })?;
    let b_embed = b_embed_map.get(b).ok_or_else(|| {
        EmbeddingError::InvalidWord(format!("Word '{}' not found in embeddings", b))
    })?;

    Ok(cosine(a_embed, b_embed))
}

/// Calculate cosine similarity between two vectors
fn cosine(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        warn!(
            "Vector length mismatch in cosine calculation: {} vs {}",
            a.len(),
            b.len()
        );
        return 0.0;
    }

    let mut dot: f64 = 0.0;
    let mut norm_a: f64 = 0.0;
    let mut norm_b: f64 = 0.0;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    norm_a = norm_a.sqrt();
    norm_b = norm_b.sqrt();

    // Handle division by zero
    if norm_a.abs() < f64::EPSILON || norm_b.abs() < f64::EPSILON {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}
