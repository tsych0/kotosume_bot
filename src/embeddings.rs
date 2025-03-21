use itertools::Itertools;
use std::collections::HashMap;
use std::fs;
use std::sync::OnceLock;

static EMBEDDINGS: OnceLock<HashMap<char, HashMap<String, Vec<f64>>>> = OnceLock::new();
const EMBEDDINGS_FILE: &str = "word2vec.txt";

fn init(file_name: &str) -> HashMap<char, HashMap<String, Vec<f64>>> {
    fs::read_to_string(&file_name)
        .unwrap()
        .replace("\r", "")
        .lines()
        .map(|l| {
            let mut word_iter = l.split_whitespace();
            let word = word_iter.next().unwrap().to_string();
            let vec = word_iter
                .map(|x| x.parse::<f64>().unwrap())
                .collect::<Vec<f64>>();
            (word, vec)
        })
        .into_iter()
        .sorted_by_key(|(key, _)| key.chars().next().unwrap()) // Sort by first character
        .chunk_by(|(key, _)| key.chars().next().unwrap()) // Group by first character
        .into_iter()
        .map(|(first_char, group)| {
            let inner_map = group.into_iter().collect::<HashMap<_, _>>();
            (first_char, inner_map)
        })
        .collect()
}

pub fn get_embeddings() -> &'static HashMap<char, HashMap<String, Vec<f64>>> {
    EMBEDDINGS.get_or_init(|| init(EMBEDDINGS_FILE))
}

pub fn is_valid_word(word: &str) -> bool {
    let first_char = word.chars().next().unwrap();
    get_embeddings()
        .get(&first_char)
        .unwrap()
        .contains_key(word)
}

pub fn get_similar_word<P>(word: &str, starting_char: char, predicate: P) -> String
where
    P: Fn(&str) -> bool,
{
    let embeddings = get_embeddings();
    let f_map = embeddings.get(&word.chars().next().unwrap()).unwrap();
    let s_map = embeddings.get(&starting_char).unwrap();

    embeddings
        .get(&starting_char)
        .unwrap()
        .keys()
        .filter(|x| predicate(*x))
        .collect::<Vec<&String>>()
        .into_iter()
        .map(|x| (similarity(word, f_map, x, s_map), x))
        .max_by(|x, y| x.0.partial_cmp(&y.0).unwrap())
        .unwrap()
        .1
        .to_string()
}

pub fn similarity(
    a: &str,
    a_embed_map: &HashMap<String, Vec<f64>>,
    b: &str,
    b_embed_map: &HashMap<String, Vec<f64>>,
) -> f64 {
    let embeddings = get_embeddings();
    cosine(a_embed_map.get(a).unwrap(), b_embed_map.get(b).unwrap())
}

fn cosine(a: &Vec<f64>, b: &Vec<f64>) -> f64 {
    assert_eq!(a.len(), b.len());

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

    dot / (norm_a * norm_b)
}
