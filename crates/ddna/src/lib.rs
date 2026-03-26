//! # VARCAVIA Data DNA (dDNA)
//!
//! Identità crittografica multi-livello per ogni dato nel sistema VARCAVIA.
//! Il dDNA accompagna ogni dato e ne certifica: autenticità, provenienza,
//! temporalità, catena di custodia e significato semantico.
//!
//! ## Componenti
//!
//! - **ContentFingerprint**: doppio hash BLAKE3 + SHA3-512
//! - **SourceIdentity**: firma Ed25519 del produttore
//! - **TemporalProof**: timestamp certificato al microsecondo
//! - **CustodyChain**: registro immutabile di tutti i passaggi
//! - **SemanticVector**: embedding vettoriale del contenuto
//! - **IntegrityProof**: hash dell'intero dDNA per auto-verifica

#![deny(clippy::all)]
#![warn(missing_docs)]

pub mod fingerprint;
pub mod identity;
pub mod temporal;
pub mod custody;
pub mod semantic;
pub mod integrity;
pub mod codec;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errori del modulo dDNA
#[derive(Error, Debug)]
pub enum DdnaError {
    /// Firma crittografica non valida
    #[error("Firma non valida: {0}")]
    InvalidSignature(String),

    /// Hash del contenuto non corrisponde
    #[error("Fingerprint non corrisponde: atteso {expected}, ottenuto {got}")]
    FingerprintMismatch {
        /// Hash atteso
        expected: String,
        /// Hash ottenuto
        got: String,
    },

    /// Timestamp non plausibile
    #[error("Timestamp non valido: {0}")]
    InvalidTimestamp(String),

    /// Catena di custodia interrotta
    #[error("Catena di custodia non valida: {0}")]
    InvalidCustody(String),

    /// Errore di serializzazione
    #[error("Errore di serializzazione: {0}")]
    SerializationError(String),

    /// Integrità del dDNA compromessa
    #[error("Integrità del dDNA compromessa")]
    IntegrityViolation,
}

/// Tipo di risultato per operazioni dDNA
pub type Result<T> = std::result::Result<T, DdnaError>;

/// Versione corrente del protocollo dDNA
pub const DDNA_VERSION: u8 = 1;

/// Il Data DNA completo — identità crittografica di un dato
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDna {
    /// Versione del protocollo dDNA
    pub version: u8,

    /// Impronta digitale del contenuto (BLAKE3 + SHA3-512)
    pub fingerprint: fingerprint::ContentFingerprint,

    /// Identità verificata del produttore
    pub source: identity::SourceIdentity,

    /// Timestamp certificato di creazione
    pub temporal: temporal::TemporalProof,

    /// Catena di custodia (ogni nodo che ha toccato il dato)
    pub custody_chain: Vec<custody::CustodyEntry>,

    /// Vettore semantico (opzionale, calcolato dall'AI)
    pub semantic_vector: Option<semantic::SemanticVector>,

    /// Hash dell'intero dDNA per auto-verifica
    pub integrity_hash: [u8; 32],
}

impl DataDna {
    /// Crea un nuovo dDNA per un dato contenuto.
    ///
    /// # Arguments
    /// * `content` - Il contenuto raw del dato
    /// * `keypair` - La coppia di chiavi Ed25519 del produttore
    ///
    /// # Returns
    /// Un nuovo `DataDna` con tutti i campi calcolati eccetto `semantic_vector`
    /// (che richiede l'AI agent Python).
    pub fn create(
        content: &[u8],
        keypair: &identity::KeyPair,
    ) -> Result<Self> {
        // 1. Calcola fingerprint
        let fp = fingerprint::ContentFingerprint::compute(content);

        // 2. Firma il fingerprint con la chiave del produttore
        let source = identity::SourceIdentity::sign(&fp, keypair)?;

        // 3. Genera prova temporale
        let temporal = temporal::TemporalProof::now();

        // 4. Inizia catena di custodia con l'azione di creazione
        let first_custody = custody::CustodyEntry::new_creation(
            &keypair.public_key_bytes(),
            keypair,
        )?;

        // 5. Crea dDNA senza integrity hash (lo calcoliamo dopo)
        let mut ddna = DataDna {
            version: DDNA_VERSION,
            fingerprint: fp,
            source,
            temporal,
            custody_chain: vec![first_custody],
            semantic_vector: None,
            integrity_hash: [0u8; 32],
        };

        // 6. Calcola integrity hash dell'intero dDNA
        ddna.integrity_hash = integrity::compute_integrity_hash(&ddna)?;

        Ok(ddna)
    }

    /// Verifica l'integrità complessiva del dDNA.
    /// Controlla: firma, timestamp plausibile, catena di custodia, integrity hash.
    pub fn verify(&self) -> Result<bool> {
        // Verifica firma del produttore
        self.source.verify(&self.fingerprint)?;

        // Verifica timestamp plausibile
        self.temporal.verify()?;

        // Verifica catena di custodia
        custody::verify_chain(&self.custody_chain)?;

        // Verifica integrity hash
        let expected_hash = integrity::compute_integrity_hash(self)?;
        if self.integrity_hash != expected_hash {
            return Err(DdnaError::IntegrityViolation);
        }

        Ok(true)
    }

    /// Verifica che il dDNA corrisponda a un dato contenuto specifico.
    pub fn verify_content(&self, content: &[u8]) -> Result<bool> {
        let computed = fingerprint::ContentFingerprint::compute(content);
        if self.fingerprint.blake3 != computed.blake3 {
            return Err(DdnaError::FingerprintMismatch {
                expected: hex::encode(self.fingerprint.blake3),
                got: hex::encode(computed.blake3),
            });
        }
        Ok(true)
    }

    /// Aggiunge un'entry alla catena di custodia (quando un nodo riceve il dato).
    pub fn add_custody(
        &mut self,
        node_id: &[u8; 32],
        action: custody::CustodyAction,
        keypair: &identity::KeyPair,
    ) -> Result<()> {
        let entry = custody::CustodyEntry::new(node_id, action, keypair)?;
        self.custody_chain.push(entry);
        // Ricalcola integrity hash
        self.integrity_hash = integrity::compute_integrity_hash(self)?;
        Ok(())
    }

    /// Imposta il vettore semantico (chiamato dopo che l'AI agent lo calcola).
    pub fn set_semantic_vector(&mut self, vector: semantic::SemanticVector) -> Result<()> {
        self.semantic_vector = Some(vector);
        // Ricalcola integrity hash
        self.integrity_hash = integrity::compute_integrity_hash(self)?;
        Ok(())
    }

    /// Restituisce l'ID univoco del dato (hash BLAKE3 esadecimale).
    pub fn id(&self) -> String {
        hex::encode(self.fingerprint.blake3)
    }

    /// Serializza il dDNA in formato MessagePack (compatto).
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        codec::serialize(self)
    }

    /// Deserializza un dDNA da bytes MessagePack.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        codec::deserialize(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify() {
        let keypair = identity::KeyPair::generate();
        let content = b"La temperatura a Roma e' 22 gradi centigradi";

        let ddna = DataDna::create(content, &keypair).expect("Creazione dDNA fallita");

        assert_eq!(ddna.version, DDNA_VERSION);
        assert!(ddna.verify().expect("Verifica fallita"));
        assert!(ddna.verify_content(content).expect("Verifica contenuto fallita"));
    }

    #[test]
    fn test_wrong_content_fails() {
        let keypair = identity::KeyPair::generate();
        let content = b"dato originale";
        let wrong_content = b"dato modificato";

        let ddna = DataDna::create(content, &keypair).unwrap();
        assert!(ddna.verify_content(wrong_content).is_err());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let keypair = identity::KeyPair::generate();
        let content = b"test data for serialization";

        let ddna = DataDna::create(content, &keypair).unwrap();
        let bytes = ddna.to_bytes().unwrap();
        let restored = DataDna::from_bytes(&bytes).unwrap();

        assert_eq!(ddna.fingerprint.blake3, restored.fingerprint.blake3);
        assert_eq!(ddna.integrity_hash, restored.integrity_hash);
    }

    #[test]
    fn test_custody_chain() {
        let producer_kp = identity::KeyPair::generate();
        let node_kp = identity::KeyPair::generate();
        let content = b"dato che passa tra nodi";

        let mut ddna = DataDna::create(content, &producer_kp).unwrap();
        assert_eq!(ddna.custody_chain.len(), 1);

        ddna.add_custody(
            &node_kp.public_key_bytes(),
            custody::CustodyAction::Received,
            &node_kp,
        ).unwrap();
        assert_eq!(ddna.custody_chain.len(), 2);

        assert!(ddna.verify().unwrap());
    }
}
