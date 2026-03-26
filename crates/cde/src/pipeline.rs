//! Pipeline CDE — Orchestrazione dei 6 stadi di pulizia.

use varcavia_ddna::DataDna;
use crate::dedup::{ExactDedupIndex, LshIndex, SemanticDedupIndex};
use crate::freshness;
use crate::normalize::{self, VufRecord};
use crate::scoring::CdeScore;
use crate::validation;
use crate::{CdeError, Result};

/// Configurazione della pipeline CDE.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Soglia LSH per near-duplicates (default 0.85)
    pub lsh_threshold: f64,
    /// Soglia per deduplicazione semantica (default 0.9 Jaccard trigram)
    pub semantic_threshold: f64,
    /// Finestra di freschezza in ore (default 24)
    pub freshness_window_hours: u32,
    /// Reputazione minima della fonte (default 0.3)
    pub min_source_reputation: f32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        PipelineConfig {
            lsh_threshold: 0.85,
            semantic_threshold: 0.9,
            freshness_window_hours: 24,
            min_source_reputation: 0.3,
        }
    }
}

/// Risultato dell'elaborazione della pipeline CDE.
#[derive(Debug)]
pub struct PipelineResult {
    /// Record normalizzato in VUF
    pub record: VufRecord,
    /// Punteggio composito
    pub score: CdeScore,
    /// Stadi superati
    pub stages_passed: Vec<String>,
    /// Avvisi (non bloccanti)
    pub warnings: Vec<String>,
}

/// Stato della pipeline CDE — mantiene gli indici di deduplicazione.
pub struct Pipeline {
    config: PipelineConfig,
    exact_index: ExactDedupIndex,
    lsh_index: LshIndex,
    semantic_index: SemanticDedupIndex,
}

impl Pipeline {
    /// Crea una nuova pipeline con la configurazione specificata.
    pub fn new(config: PipelineConfig) -> Self {
        let lsh_threshold = config.lsh_threshold;
        let semantic_threshold = config.semantic_threshold;
        Pipeline {
            config,
            exact_index: ExactDedupIndex::new(),
            lsh_index: LshIndex::new(128, lsh_threshold),
            semantic_index: SemanticDedupIndex::new(semantic_threshold),
        }
    }

    /// Processa un dato attraverso tutti i 6 stadi della pipeline.
    pub fn process(
        &mut self,
        data: &[u8],
        ddna: &DataDna,
        domain: &str,
    ) -> Result<PipelineResult> {
        let mut stages_passed = Vec::new();
        let mut warnings = Vec::new();
        let data_id = ddna.id();

        // STADIO 1: Deduplicazione hash esatto
        if self.exact_index.check(&ddna.fingerprint.blake3).is_some() {
            return Err(CdeError::DuplicateData(format!(
                "Duplicato esatto: {}",
                data_id
            )));
        }
        stages_passed.push("dedup_exact".into());

        // STADIO 2: Deduplicazione near-duplicate (LSH)
        if let Some((existing_id, similarity)) = self.lsh_index.check_near_duplicate(data) {
            warnings.push(format!(
                "Near-duplicate trovato: {} (similarità: {similarity:.2})",
                existing_id
            ));
        }
        stages_passed.push("dedup_lsh".into());

        // STADIO 3: Deduplicazione semantica (character trigram Jaccard)
        // Rust-native: confronta trigrammi di caratteri per catturare testi simili.
        // TODO: sostituire con embedding ONNX (all-MiniLM-L6-v2) per similarità semantica reale.
        if let Ok(text) = std::str::from_utf8(data) {
            if let Some((existing_id, similarity)) =
                self.semantic_index.check_semantic_duplicate(text)
            {
                warnings.push(format!(
                    "Duplicato semantico: {} (similarità: {similarity:.2})",
                    existing_id
                ));
            }
        }
        stages_passed.push("dedup_semantic".into());

        // STADIO 4: Validazione fonte
        let source_result = validation::validate_source(
            data,
            ddna,
            self.config.min_source_reputation,
        )?;
        if !source_result.reputation_sufficient {
            warnings.push(format!(
                "Reputazione fonte bassa: {:.2}",
                source_result.source_reputation
            ));
        }
        stages_passed.push("validation".into());

        // STADIO 5: Normalizzazione in VUF
        let record = normalize::normalize(data, domain)?;
        stages_passed.push("normalization".into());

        // STADIO 6: Scoring
        let freshness_window_us =
            self.config.freshness_window_hours as i64 * 3600 * 1_000_000;
        let freshness_result = freshness::check_freshness(ddna, freshness_window_us);

        // Coerenza: se non ci sono warning semantici, coerenza = 1.0
        let coherence = if warnings.iter().any(|w| w.contains("semantico")) {
            0.5
        } else {
            1.0
        };

        let score = CdeScore::compute(
            source_result.source_reputation as f64,
            coherence,
            freshness_result.freshness_score,
            1, // Validazioni iniziali: 1 (la nostra)
        );
        stages_passed.push("scoring".into());

        // Registra negli indici
        self.exact_index
            .insert(ddna.fingerprint.blake3, data_id.clone());
        self.lsh_index.insert(data_id.clone(), data);
        if let Ok(text) = std::str::from_utf8(data) {
            self.semantic_index.insert(data_id, text);
        }

        Ok(PipelineResult {
            record,
            score,
            stages_passed,
            warnings,
        })
    }

    /// Numero di dati processati.
    pub fn data_count(&self) -> usize {
        self.exact_index.len()
    }
}

/// Estrae claim fattuali da un testo lungo.
/// Splitta in frasi e filtra quelle che contengono pattern fattuali.
pub fn extract_claims(text: &str) -> Vec<String> {
    let mut claims = Vec::new();

    // Split per frasi (. ! ?)
    for raw in text.split(['.', '!', '?']) {
        let sentence = raw.trim();
        if sentence.len() < 15 {
            continue;
        }

        let lower = sentence.to_lowercase();

        // Contiene un numero?
        let has_number = sentence.chars().any(|c| c.is_ascii_digit());

        // Contiene un nome proprio? (parola che inizia con maiuscola, non a inizio frase)
        let words: Vec<&str> = sentence.split_whitespace().collect();
        let has_proper_noun = words.iter().skip(1).any(|w| {
            w.chars().next().is_some_and(|c| c.is_uppercase())
        });

        // Contiene pattern fattuali?
        let has_pattern = lower.contains(" is ")
            || lower.contains(" are ")
            || lower.contains(" was ")
            || lower.contains(" were ")
            || lower.contains(" has ")
            || lower.contains(" have ")
            || lower.contains(" had ")
            || lower.contains(" contains ")
            || lower.contains(" measures ")
            || lower.contains(" weighs ")
            || lower.contains(" consists ")
            || lower.contains(" comprises ")
            || lower.contains(" founded ")
            || lower.contains(" discovered ")
            || lower.contains(" invented ");

        if has_pattern && (has_number || has_proper_noun) {
            // Ricostruisci con punto finale
            let claim = format!("{sentence}.");
            if !claims.contains(&claim) {
                claims.push(claim);
            }
        }
    }

    claims
}

#[cfg(test)]
mod tests {
    use super::*;
    use varcavia_ddna::identity::KeyPair;

    fn create_test_ddna(content: &[u8]) -> (DataDna, KeyPair) {
        let kp = KeyPair::generate();
        let ddna = DataDna::create(content, &kp).unwrap();
        (ddna, kp)
    }

    #[test]
    fn test_pipeline_basic() {
        let mut pipeline = Pipeline::new(PipelineConfig::default());
        let data = b"La temperatura a Roma e' 22 gradi";
        let (ddna, _kp) = create_test_ddna(data);
        let result = pipeline.process(data, &ddna, "climate").unwrap();
        assert!(result.stages_passed.contains(&"dedup_semantic".to_string()));
        assert!(result.score.overall > 0.0);
    }

    #[test]
    fn test_pipeline_duplicate_rejected() {
        let mut pipeline = Pipeline::new(PipelineConfig::default());
        let data = b"dato duplicato";
        let (ddna, _kp) = create_test_ddna(data);
        pipeline.process(data, &ddna, "test").unwrap();
        let result = pipeline.process(data, &ddna, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_different_data_ok() {
        let mut pipeline = Pipeline::new(PipelineConfig::default());
        let (ddna1, _) = create_test_ddna(b"dato uno completamente diverso");
        let (ddna2, _) = create_test_ddna(b"dato due totalmente differente");
        assert!(pipeline.process(b"dato uno completamente diverso", &ddna1, "test").is_ok());
        assert!(pipeline.process(b"dato due totalmente differente", &ddna2, "test").is_ok());
        assert_eq!(pipeline.data_count(), 2);
    }

    #[test]
    fn test_pipeline_data_count() {
        let mut pipeline = Pipeline::new(PipelineConfig::default());
        assert_eq!(pipeline.data_count(), 0);
        let (ddna, _) = create_test_ddna(b"test pipeline count");
        pipeline.process(b"test pipeline count", &ddna, "test").unwrap();
        assert_eq!(pipeline.data_count(), 1);
    }

    #[test]
    fn test_extract_claims() {
        let text = "Earth is the third planet from the Sun. It has a radius of 6371 km. The sky is blue. Water has a boiling point of 100 degrees Celsius. Hi.";
        let claims = extract_claims(text);
        assert!(claims.len() >= 2);
        assert!(claims.iter().any(|c| c.contains("Earth")));
        assert!(claims.iter().any(|c| c.contains("100")));
    }

    #[test]
    fn test_extract_claims_empty() {
        let claims = extract_claims("hello world");
        assert!(claims.is_empty());
    }

    #[test]
    fn test_pipeline_semantic_warning() {
        let mut pipeline = Pipeline::new(PipelineConfig {
            semantic_threshold: 0.6, // Lower threshold to catch similar texts
            ..Default::default()
        });
        let data1 = b"La temperatura a Roma oggi e ventitreesima gradi celsius";
        let data2 = b"La temperatura a Roma oggi e ventiquattro gradi celsius";
        let (ddna1, _) = create_test_ddna(data1);
        let (ddna2, _) = create_test_ddna(data2);
        let r1 = pipeline.process(data1, &ddna1, "climate").unwrap();
        assert!(r1.warnings.is_empty());
        let r2 = pipeline.process(data2, &ddna2, "climate").unwrap();
        // Second insert should have a semantic similarity warning
        assert!(r2.warnings.iter().any(|w| w.contains("semantico")));
    }
}
