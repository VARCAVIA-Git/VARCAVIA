//! Semantic Vector — Embedding vettoriale del contenuto di un dato.
//! Calcolato dall'AI agent Python (all-MiniLM-L6-v2 via ONNX Runtime),
//! trasportato e verificato in Rust.

use half::f16;
use serde::{Deserialize, Serialize};

/// Vettore semantico che rappresenta il significato del dato.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticVector {
    /// ID del modello usato per generare l'embedding
    pub model_id: String,
    /// Numero di dimensioni del vettore
    pub dimensions: u16,
    /// Valori del vettore in f16 (mezzo la memoria di f32)
    pub values: Vec<f16>,
}

impl SemanticVector {
    /// Crea un nuovo SemanticVector da valori f32 (convertiti a f16).
    pub fn from_f32(model_id: &str, values: &[f32]) -> Self {
        SemanticVector {
            model_id: model_id.to_string(),
            dimensions: values.len() as u16,
            values: values.iter().map(|&v| f16::from_f32(v)).collect(),
        }
    }

    /// Converte il vettore in f32 per i calcoli.
    pub fn to_f32(&self) -> Vec<f32> {
        self.values.iter().map(|v| v.to_f32()).collect()
    }

    /// Calcola la similarità coseno con un altro vettore.
    pub fn cosine_similarity(&self, other: &SemanticVector) -> f32 {
        if self.dimensions != other.dimensions {
            return 0.0;
        }
        let a = self.to_f32();
        let b = other.to_f32();

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_f32() {
        let values = vec![0.1, 0.2, 0.3];
        let sv = SemanticVector::from_f32("test-model", &values);
        assert_eq!(sv.dimensions, 3);
        assert_eq!(sv.model_id, "test-model");
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let values = vec![1.0, 0.0, 0.0];
        let sv1 = SemanticVector::from_f32("m", &values);
        let sv2 = SemanticVector::from_f32("m", &values);
        let sim = sv1.cosine_similarity(&sv2);
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let sv1 = SemanticVector::from_f32("m", &[1.0, 0.0]);
        let sv2 = SemanticVector::from_f32("m", &[0.0, 1.0]);
        let sim = sv1.cosine_similarity(&sv2);
        assert!(sim.abs() < 0.01);
    }
}
