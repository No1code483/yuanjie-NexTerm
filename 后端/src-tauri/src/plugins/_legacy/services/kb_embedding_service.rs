use std::collections::HashMap;

/// Tokenize text into a list of terms.
/// Splits Chinese characters individually and extracts alphanumeric words.
fn tokenize(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current_word = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() {
            current_word.push(ch);
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            if !current_word.is_empty() {
                tokens.push(current_word.to_lowercase());
                current_word.clear();
            }
        } else {
            // Chinese, Japanese, or punctuation — treat each as a separate token
            if !current_word.is_empty() {
                tokens.push(current_word.to_lowercase());
                current_word.clear();
            }
            if !ch.is_ascii_punctuation() {
                tokens.push(ch.to_string());
            }
        }
    }
    if !current_word.is_empty() {
        tokens.push(current_word.to_lowercase());
    }
    tokens
}

/// Build a vocabulary from all documents and compute document frequencies.
fn build_vocab(docs: &[Vec<String>]) -> (Vec<String>, HashMap<String, usize>) {
    let mut df: HashMap<String, usize> = HashMap::new();
    for doc in docs {
        let mut seen: HashMap<String, bool> = HashMap::new();
        for term in doc {
            if !seen.contains_key(term) {
                seen.insert(term.clone(), true);
                *df.entry(term.clone()).or_insert(0) += 1;
            }
        }
    }
    let mut vocab: Vec<String> = df.keys().cloned().collect();
    vocab.sort();
    (vocab, df)
}

/// Compute TF-IDF vector for a single document given the vocabulary and document frequencies.
fn compute_tfidf(doc: &[String], vocab: &[String], df: &HashMap<String, usize>, num_docs: usize) -> Vec<f32> {
    // Compute term frequencies
    let mut tf: HashMap<String, f32> = HashMap::new();
    let len = doc.len().max(1) as f32;
    for term in doc {
        *tf.entry(term.clone()).or_insert(0.0) += 1.0;
    }
    for v in tf.values_mut() {
        *v /= len;
    }

    let mut vec = Vec::with_capacity(vocab.len());
    for term in vocab {
        let tf_val = tf.get(term).copied().unwrap_or(0.0);
        let df_val = *df.get(term).unwrap_or(&1) as f32;
        let idf = ((num_docs as f32 + 1.0) / (df_val + 1.0)).ln() + 1.0;
        vec.push(tf_val * idf);
    }
    vec
}

/// Generate a TF-IDF embedding vector for a single text.
/// If a corpus context is provided (for vocabulary building), use it.
/// Otherwise, use a self-contained approach.
pub fn generate_embedding(text: &str) -> Vec<f32> {
    let tokens = tokenize(text);
    // Self-contained: use the single document as its own corpus
    let docs = vec![tokens.clone()];
    let (vocab, df) = build_vocab(&docs);
    compute_tfidf(&tokens, &vocab, &df, 1)
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for i in 0..len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    // Remaining dimensions contribute 0 to dot product
    for i in len..a.len() {
        norm_a += a[i] * a[i];
    }
    for i in len..b.len() {
        norm_b += b[i] * b[i];
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

/// Generate TF-IDF embeddings for a batch of texts, sharing a common vocabulary.
pub fn generate_embeddings_batch(texts: &[String]) -> Vec<Vec<f32>> {
    if texts.is_empty() {
        return Vec::new();
    }
    let tokenized: Vec<Vec<String>> = texts.iter().map(|t| tokenize(t)).collect();
    let (vocab, df) = build_vocab(&tokenized);
    let num_docs = texts.len();
    tokenized
        .iter()
        .map(|tokens| compute_tfidf(tokens, &vocab, &df, num_docs))
        .collect()
}

/// Search for the top-N most similar documents to a query.
/// Returns (index, score) pairs sorted by descending score.
pub fn search_similar(
    query: &str,
    documents: &[String],
    limit: usize,
) -> Vec<(usize, f32)> {
    if documents.is_empty() {
        return Vec::new();
    }
    let mut all_texts: Vec<String> = documents.to_vec();
    all_texts.push(query.to_string());
    let embeddings = generate_embeddings_batch(&all_texts);
    if embeddings.is_empty() {
        return Vec::new();
    }
    let query_embedding = embeddings.last().unwrap();
    let doc_embeddings = &embeddings[..embeddings.len() - 1];

    let mut scores: Vec<(usize, f32)> = doc_embeddings
        .iter()
        .enumerate()
        .map(|(i, emb)| (i, cosine_similarity(query_embedding, emb)))
        .collect();
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(limit);
    scores
}