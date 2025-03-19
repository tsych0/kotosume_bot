use std::collections::HashMap;
use std::fs;
use std::sync::OnceLock;

static EMBEDDINGS: OnceLock<HashMap<String, Vec<f64>>> = OnceLock::new();
const EMBEDDINGS_FILE: &str = "word2vec.txt";

fn init(file_name: &str) -> HashMap<String, Vec<f64>> {
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
        .collect()
}

fn get_embeddings() -> &'static HashMap<String, Vec<f64>> {
    EMBEDDINGS.get_or_init(|| init(EMBEDDINGS_FILE))
}

pub fn is_valid_word(word: &str) -> bool {
    get_embeddings().contains_key(word)
}

pub fn get_similar_word<P>(word: &str, predicate: P) -> String
where
    P: Fn(&str) -> bool,
{
    let embeddings = get_embeddings();
    embeddings
        .keys()
        .filter(|x| predicate(*x))
        .collect::<Vec<&String>>()
        .into_iter()
        .map(|x| (similarity(word, x), x))
        .max_by(|x, y| x.0.partial_cmp(&y.0).unwrap())
        .unwrap()
        .1
        .to_string()
}

pub fn similarity(a: &str, b: &str) -> f64 {
    let embeddings = get_embeddings();
    cosine(embeddings.get(a).unwrap(), embeddings.get(b).unwrap())
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
