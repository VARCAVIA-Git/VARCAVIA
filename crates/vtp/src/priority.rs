//! Semantic Priority Queuing — Priorità basata sul dominio e contesto del dato.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Livello di priorità semantica di un pacchetto VTP.
/// La priorità determina l'ordine di trasmissione nella coda di uscita.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SemanticPriority {
    /// Emergenza: dati clinici critici, allerta disastri (trasmessi immediatamente)
    Critical = 0,
    /// Alta: dati finanziari in tempo reale, aggiornamenti di sicurezza
    High = 1,
    /// Normale: dati di ricerca, aggiornamenti standard
    Normal = 2,
    /// Bassa: statistiche aggregate, dati storici
    Low = 3,
    /// Background: backup, pre-posizionamento predittivo
    Background = 4,
}

impl SemanticPriority {
    /// Inferisce la priorità dal dominio del dato.
    pub fn from_domain(domain: &str) -> Self {
        match domain.to_lowercase().as_str() {
            "emergency" | "disaster" | "critical_health" => SemanticPriority::Critical,
            "health" | "finance" | "security" => SemanticPriority::High,
            "climate" | "science" | "education" => SemanticPriority::Normal,
            "statistics" | "historical" | "archive" => SemanticPriority::Low,
            _ => SemanticPriority::Normal,
        }
    }

    /// Restituisce il valore numerico (più basso = più prioritario).
    pub fn value(&self) -> u8 {
        *self as u8
    }
}

impl Ord for SemanticPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        // Invertito: Critical (0) ha priorità maggiore
        (*self as u8).cmp(&(*other as u8))
    }
}

impl PartialOrd for SemanticPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(SemanticPriority::Critical < SemanticPriority::High);
        assert!(SemanticPriority::High < SemanticPriority::Normal);
    }

    #[test]
    fn test_from_domain() {
        assert_eq!(SemanticPriority::from_domain("emergency"), SemanticPriority::Critical);
        assert_eq!(SemanticPriority::from_domain("health"), SemanticPriority::High);
        assert_eq!(SemanticPriority::from_domain("climate"), SemanticPriority::Normal);
        assert_eq!(SemanticPriority::from_domain("unknown"), SemanticPriority::Normal);
    }
}
