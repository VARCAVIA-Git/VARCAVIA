//! Source Identity — Firma Ed25519 e identità del produttore del dato.

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey, Signature};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::fingerprint::ContentFingerprint;
use crate::{DdnaError, Result};

/// Tipo di identità del produttore.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IdentityType {
    /// Istituzione verificata (governo, università, azienda certificata)
    Institutional,
    /// Identità pseudonima con reputazione accumulata
    Pseudonymous,
}

/// Identità verificata del produttore di un dato.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceIdentity {
    /// Chiave pubblica Ed25519 (32 bytes)
    pub public_key: [u8; 32],
    /// Firma del ContentFingerprint (64 bytes)
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    /// Tipo di identità
    pub identity_type: IdentityType,
    /// Punteggio di reputazione corrente (0.0 - 1.0)
    pub reputation_score: f32,
}

/// Coppia di chiavi Ed25519 per firmare i dati.
pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    /// Genera una nuova coppia di chiavi casuale.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        KeyPair { signing_key }
    }

    /// Crea un KeyPair da una chiave privata raw (32 bytes).
    pub fn from_bytes(secret: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(secret);
        KeyPair { signing_key }
    }

    /// Restituisce la chiave pubblica come array di 32 bytes.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Restituisce i bytes della chiave privata (ATTENZIONE: sensibile).
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Firma un messaggio arbitrario.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let sig = self.signing_key.sign(message);
        sig.to_bytes()
    }
}

impl SourceIdentity {
    /// Crea una nuova SourceIdentity firmando un ContentFingerprint.
    pub fn sign(fingerprint: &ContentFingerprint, keypair: &KeyPair) -> Result<Self> {
        // Il messaggio da firmare è la concatenazione dei due hash
        let mut message = Vec::with_capacity(96);
        message.extend_from_slice(&fingerprint.blake3);
        message.extend_from_slice(&fingerprint.sha3_512);

        let signature = keypair.sign(&message);

        Ok(SourceIdentity {
            public_key: keypair.public_key_bytes(),
            signature,
            identity_type: IdentityType::Pseudonymous,
            reputation_score: 0.5, // Default per nuova identità
        })
    }

    /// Verifica che la firma corrisponda al fingerprint e alla chiave pubblica.
    pub fn verify(&self, fingerprint: &ContentFingerprint) -> Result<bool> {
        let verifying_key = VerifyingKey::from_bytes(&self.public_key)
            .map_err(|e| DdnaError::InvalidSignature(format!("Chiave pubblica invalida: {e}")))?;

        let signature = Signature::from_bytes(&self.signature);

        let mut message = Vec::with_capacity(96);
        message.extend_from_slice(&fingerprint.blake3);
        message.extend_from_slice(&fingerprint.sha3_512);

        verifying_key
            .verify(&message, &signature)
            .map_err(|e| DdnaError::InvalidSignature(format!("Firma non valida: {e}")))?;

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = KeyPair::generate();
        let pubkey = kp.public_key_bytes();
        assert_eq!(pubkey.len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let kp = KeyPair::generate();
        let fp = ContentFingerprint::compute(b"test data");
        let source = SourceIdentity::sign(&fp, &kp).unwrap();
        assert!(source.verify(&fp).unwrap());
    }

    #[test]
    fn test_wrong_key_fails() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let fp = ContentFingerprint::compute(b"test data");
        let source = SourceIdentity::sign(&fp, &kp1).unwrap();

        // Modifica la chiave pubblica con quella di kp2
        let mut wrong_source = source;
        wrong_source.public_key = kp2.public_key_bytes();
        assert!(wrong_source.verify(&fp).is_err());
    }

    #[test]
    fn test_keypair_roundtrip() {
        let kp = KeyPair::generate();
        let secret = kp.secret_bytes();
        let kp2 = KeyPair::from_bytes(&secret);
        assert_eq!(kp.public_key_bytes(), kp2.public_key_bytes());
    }

    #[test]
    fn test_wrong_fingerprint_fails() {
        let kp = KeyPair::generate();
        let fp1 = ContentFingerprint::compute(b"data A");
        let fp2 = ContentFingerprint::compute(b"data B");
        let source = SourceIdentity::sign(&fp1, &kp).unwrap();
        assert!(source.verify(&fp2).is_err());
    }

    #[test]
    fn test_default_reputation() {
        let kp = KeyPair::generate();
        let fp = ContentFingerprint::compute(b"test");
        let source = SourceIdentity::sign(&fp, &kp).unwrap();
        assert_eq!(source.reputation_score, 0.5);
        assert_eq!(source.identity_type, IdentityType::Pseudonymous);
    }
}
