//! Pipeline CDE — Orchestrazione dei 6 stadi di pulizia.

use varcavia_ddna::DataDna;
use crate::dedup::{ExactDedupIndex, LshIndex};
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
    /// Soglia per deduplicazione semantica (default 0.1 distanza coseno)
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
            semantic_threshold: 0.1,
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
}

impl Pipeline {
    /// Crea una nuova pipeline con la configurazione specificata.
    pub fn new(config: PipelineConfig) -> Self {
        let lsh_threshold = config.lsh_threshold;
        Pipeline {
            config,
            exact_index: ExactDedupIndex::new(),
            lsh_index: LshIndex::new(128, lsh_threshold),
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

        // STADIO 3: Deduplicazione semantica (richiede Python agent)
        // Per ora: skip, segna come TODO
        stages_passed.push("dedup_semantic_skipped".into());

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

        let score = CdeScore::compute(
            source_result.source_reputation as f64,
            1.0, // Coerenza: TODO, richiede AI agent
            freshness_result.freshness_score,
            1, // Validazioni iniziali: 1 (la nostra)
        );
        stages_passed.push("scoring".into());

        // Registra negli indici
        self.exact_index.insert(ddna.fingerprint.blake3, data_id.clone());
        self.lsh_index.insert(data_id, data);

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
        assert!(!result.stages_passed.is_empty());
        assert!(result.score.overall > 0.0);
    }

    #[test]
    fn test_pipeline_duplicate_rejected() {
        let mut pipeline = Pipeline::new(PipelineConfig::default());
        let data = b"dato duplicato";
        let (ddna, _kp) = create_test_ddna(data);
        pipeline.process(data, &ddna, "test").unwrap();
        // Second time: should fail
        let result = pipeline.process(data, &ddna, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_different_data_ok() {
        let mut pipeline = Pipeline::new(PipelineConfig::default());
        let (ddna1, _) = create_test_ddna(b"dato uno");
        let (ddna2, _) = create_test_ddna(b"dato due");
        assert!(pipeline.process(b"dato uno", &ddna1, "test").is_ok());
        assert!(pipeline.process(b"dato due", &ddna2, "test").is_ok());
        assert_eq!(pipeline.data_count(), 2);
    }

    #[test]
    fn test_pipeline_data_count() {
        let mut pipeline = Pipeline::new(PipelineConfig::default());
        assert_eq!(pipeline.data_count(), 0);
        let (ddna, _) = create_test_ddna(b"test");
        pipeline.process(b"test", &ddna, "test").unwrap();
        assert_eq!(pipeline.data_count(), 1);
    }
}
