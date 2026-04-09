use std::collections::HashMap;

/// Reciprocal Rank Fusion — combines multiple ranked lists into one.
/// Used by MAGMA to fuse semantic, temporal, causal, and entity retrieval signals.
/// k = 60 is the standard RRF constant.
pub fn reciprocal_rank_fusion(
    ranked_lists: &[Vec<(String, f32)>],
    k: f64,
    top_n: usize,
) -> Vec<(String, f64)> {
    let mut scores: HashMap<String, f64> = HashMap::new();

    for list in ranked_lists {
        for (rank, (id, _score)) in list.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f64 + 1.0);
            *scores.entry(id.clone()).or_default() += rrf_score;
        }
    }

    let mut fused: Vec<(String, f64)> = scores.into_iter().collect();
    fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    fused.truncate(top_n);
    fused
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_basic() {
        let list1 = vec![
            ("a".to_string(), 0.9),
            ("b".to_string(), 0.7),
            ("c".to_string(), 0.5),
        ];
        let list2 = vec![
            ("b".to_string(), 0.8),
            ("a".to_string(), 0.6),
            ("d".to_string(), 0.4),
        ];

        let fused = reciprocal_rank_fusion(&[list1, list2], 60.0, 3);
        assert_eq!(fused.len(), 3);

        // "a" is rank 1 in list1 (1/61) and rank 2 in list2 (1/62) = 0.01639 + 0.01613 = 0.03252
        // "b" is rank 2 in list1 (1/62) and rank 1 in list2 (1/61) = same total
        // Both should be approximately equal and highest
        let ids: Vec<&str> = fused.iter().map(|(id, _)| id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
    }

    #[test]
    fn test_rrf_single_list() {
        let list = vec![
            ("x".to_string(), 1.0),
            ("y".to_string(), 0.5),
        ];

        let fused = reciprocal_rank_fusion(&[list], 60.0, 5);
        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].0, "x");
    }
}
