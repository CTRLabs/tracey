/// In-memory vector index for semantic search
/// Uses brute-force cosine similarity (efficient for <100K vectors)
pub struct VectorIndex {
    entries: Vec<VectorEntry>,
}

struct VectorEntry {
    id: String,
    embedding: Vec<f32>,
}

impl VectorIndex {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn insert(&mut self, id: &str, embedding: Vec<f32>) {
        // Update if exists
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.embedding = embedding;
            return;
        }
        self.entries.push(VectorEntry {
            id: id.to_string(),
            embedding,
        });
    }

    pub fn remove(&mut self, id: &str) {
        self.entries.retain(|e| e.id != id);
    }

    /// Search for the top-k most similar vectors by cosine similarity
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = self
            .entries
            .iter()
            .map(|entry| {
                let sim = cosine_similarity(query, &entry.embedding);
                (entry.id.clone(), sim)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c)).abs() < 0.001);
    }

    #[test]
    fn test_vector_index_search() {
        let mut index = VectorIndex::new();
        index.insert("doc1", vec![1.0, 0.0, 0.0]);
        index.insert("doc2", vec![0.9, 0.1, 0.0]);
        index.insert("doc3", vec![0.0, 1.0, 0.0]);

        let results = index.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "doc1"); // Most similar
        assert_eq!(results[1].0, "doc2"); // Second most similar
    }
}
