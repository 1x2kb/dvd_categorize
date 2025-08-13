//! Provides vector similarity functionality for types that can be compared using vector embeddings.

/// A trait for types that can calculate similarity with a query vector.
///
/// This trait provides methods for calculating the similarity between a type's internal
/// vector representation and an external query vector. The similarity is typically
/// a value between -1.0 and 1.0, where 1.0 means identical vectors and -1.0 means
/// completely opposite vectors.
pub trait VectorSimilarity {
    /// Calculates the cosine similarity between the type's vector and a query vector.
    ///
    /// # Arguments
    /// * `query_embedding` - A slice of f32 representing the query vector
    ///
    /// # Returns
    /// * `Some(f32)` - The cosine similarity score if both vectors are valid and of the same length
    /// * `None` - If either vector is invalid or lengths don't match
    fn cosine_similarity(&self, query_embedding: &[f32]) -> Option<f32>;

    /// Calculates the cosine similarity between two vectors.
    ///
    /// This is a utility function that can be used by implementors of this trait.
    ///
    /// Order of a and b do not matter.
    ///
    /// # Arguments
    /// * `a` - First vector
    /// * `b` - Second vector
    ///
    /// # Returns
    /// * `Some(f32)` - The cosine similarity score if both vectors are valid and of the same length
    /// * `None` - If either vector is empty or lengths don't match
    fn cosine_similarity_vectors(a: &[f32], b: &[f32]) -> Option<f32> {
        if a.is_empty() || b.is_empty() || a.len() != b.len() {
            return None;
        }

        let dot_product: f32 = a
            .iter()
            .zip(b.iter())
            .map(|(a, b)| a * b)
            .sum();

        let a_norm: f32 = a
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt();

        let b_norm: f32 = b
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt();

        if a_norm == 0.0 || b_norm == 0.0 {
            return None;
        }

        Some(dot_product / (a_norm * b_norm))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestVector {
        vec: Vec<f32>,
    }

    impl VectorSimilarity for TestVector {
        fn cosine_similarity(&self, query_embedding: &[f32]) -> Option<f32> {
            Self::cosine_similarity_vectors(
                &self.vec,
                query_embedding,
            )
        }
    }

    #[test]
    fn test_cosine_similarity() {
        let v1 = TestVector {
            vec: vec![1.0, 0.0],
        };
        let v2 = vec![1.0, 0.0];
        assert_eq!(
            v1.cosine_similarity(&v2),
            Some(1.0)
        );

        let v3 = vec![0.0, 1.0];
        assert_eq!(
            v1.cosine_similarity(&v3),
            Some(0.0)
        );

        let v4 = vec![-1.0, 0.0];
        assert_eq!(
            v1.cosine_similarity(&v4),
            Some(-1.0)
        );
    }

    #[test]
    fn test_invalid_inputs() {
        let v = TestVector {
            vec: vec![1.0, 0.0],
        };
        assert_eq!(
            v.cosine_similarity(&[]),
            None
        );
        assert_eq!(
            v.cosine_similarity(&[1.0]),
            None
        );
        assert_eq!(
            v.cosine_similarity(&[1.0, 0.0, 0.0]),
            None
        );
    }
}
