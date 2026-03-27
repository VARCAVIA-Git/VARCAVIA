//! Trust Tier System — VERIT Protocol trust scoring.
//!
//! Fatti progrediscono da T0 (non attestato) a T4 (massima fiducia)
//! in base ad attestazioni, contraddizioni, eta e domanda.

use serde::{Deserialize, Serialize};

/// Livelli di fiducia VERIT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VeritTier {
    T0,
    T1,
    T2,
    T3,
    T4,
}

impl std::fmt::Display for VeritTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VeritTier::T0 => write!(f, "T0"),
            VeritTier::T1 => write!(f, "T1"),
            VeritTier::T2 => write!(f, "T2"),
            VeritTier::T3 => write!(f, "T3"),
            VeritTier::T4 => write!(f, "T4"),
        }
    }
}

/// Record di fiducia per un fatto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRecord {
    pub tier: VeritTier,
    pub attestations: Vec<Attestation>,
    pub contradictions: Vec<Contradiction>,
    pub query_count: u64,
    pub first_seen_us: i64,
    pub last_updated_us: i64,
}

impl TrustRecord {
    /// Crea un TrustRecord vuoto (T0).
    pub fn new(now_us: i64) -> Self {
        TrustRecord {
            tier: VeritTier::T0,
            attestations: Vec::new(),
            contradictions: Vec::new(),
            query_count: 0,
            first_seen_us: now_us,
            last_updated_us: now_us,
        }
    }

    /// Peso totale delle attestazioni.
    pub fn authority_score(&self) -> f64 {
        self.attestations.iter().map(|a| a.source_tier.weight()).sum()
    }

    /// Numero di domini distinti nelle attestazioni.
    pub fn distinct_domains(&self) -> usize {
        let mut domains = std::collections::HashSet::new();
        for a in &self.attestations {
            domains.insert(a.domain.clone());
        }
        domains.len()
    }

    /// Conta attestazioni Institutional o PeerReviewed.
    pub fn high_authority_count(&self) -> usize {
        self.attestations.iter().filter(|a| {
            matches!(a.source_tier, SourceTier::Institutional | SourceTier::PeerReviewed)
        }).count()
    }

    /// Eta del fatto in giorni.
    pub fn age_days(&self, now_us: i64) -> f64 {
        let elapsed_us = now_us.saturating_sub(self.first_seen_us);
        elapsed_us as f64 / (86400.0 * 1_000_000.0)
    }

    /// Conta attestazioni per tipo di fonte.
    pub fn source_breakdown(&self) -> SourceBreakdown {
        let mut b = SourceBreakdown::default();
        for a in &self.attestations {
            match a.source_tier {
                SourceTier::Institutional => b.institutional += 1,
                SourceTier::PeerReviewed => b.peer_reviewed += 1,
                SourceTier::MainstreamMedia => b.mainstream_media += 1,
                SourceTier::Website => b.website += 1,
                SourceTier::Anonymous => b.anonymous += 1,
            }
        }
        b
    }
}

/// Breakdown delle fonti per tipo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceBreakdown {
    pub institutional: u32,
    pub peer_reviewed: u32,
    pub mainstream_media: u32,
    pub website: u32,
    pub anonymous: u32,
}

/// Attestazione da una fonte.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    pub source_pubkey: String,
    pub domain: String,
    pub source_tier: SourceTier,
    pub timestamp_us: i64,
}

/// Tipo di fonte con peso associato.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceTier {
    Institutional,
    PeerReviewed,
    MainstreamMedia,
    Website,
    Anonymous,
}

impl SourceTier {
    pub fn weight(&self) -> f64 {
        match self {
            SourceTier::Institutional => 10.0,
            SourceTier::PeerReviewed => 5.0,
            SourceTier::MainstreamMedia => 3.0,
            SourceTier::Website => 1.0,
            SourceTier::Anonymous => 0.5,
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "institutional" => SourceTier::Institutional,
            "peer_reviewed" | "peerreviewed" => SourceTier::PeerReviewed,
            "mainstream_media" | "mainstreammedia" | "media" => SourceTier::MainstreamMedia,
            "website" | "web" => SourceTier::Website,
            _ => SourceTier::Anonymous,
        }
    }
}

/// Contraddizione registrata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub conflicting_fact_id: String,
    pub description: String,
    pub source_pubkey: String,
    pub timestamp_us: i64,
}

/// Etichetta human-readable per ogni tier.
pub fn tier_label(tier: &VeritTier) -> &'static str {
    match tier {
        VeritTier::T0 => "Unattested",
        VeritTier::T1 => "Attested",
        VeritTier::T2 => "Corroborated",
        VeritTier::T3 => "Authoritative",
        VeritTier::T4 => "Canonical",
    }
}

/// Calcola il numero effettivo di fonti indipendenti.
/// Fonti dallo stesso dominio contribuiscono 0.3, da domini diversi 1.0.
pub fn compute_independence(attestations: &[Attestation]) -> f64 {
    if attestations.is_empty() {
        return 0.0;
    }
    if attestations.len() == 1 {
        return 1.0;
    }

    let mut total = 0.0;
    let mut counted = vec![false; attestations.len()];

    for i in 0..attestations.len() {
        if counted[i] {
            continue;
        }
        counted[i] = true;
        total += 1.0; // Prima attestazione in un gruppo conta 1.0

        for j in (i + 1)..attestations.len() {
            if counted[j] {
                continue;
            }
            if attestations[i].domain == attestations[j].domain {
                // Stesso dominio — contributo parziale
                counted[j] = true;
                total += 0.3;
            }
        }
    }

    // Fonti da domini non ancora contati
    total += counted.iter().filter(|&&c| !c).count() as f64;

    (total * 10.0_f64).round() / 10.0
}

/// Estrae soggetto e numero da un fatto per contradiction detection.
/// Pattern: "SUBJECT is/has/was NUMBER UNIT"
/// Restituisce (soggetto_normalizzato, numero) se trovato.
pub fn extract_subject_number(fact: &str) -> Option<(String, f64)> {
    let lower = fact.to_lowercase();

    // Trova il verbo separatore
    let verbs = [" is ", " has ", " was ", " are ", " have ", " measures "];
    let (subject_part, rest) = verbs.iter()
        .filter_map(|v| {
            lower.find(v).map(|pos| {
                let subj = &fact[..pos];
                let rest = &fact[pos + v.len()..];
                (subj.to_string(), rest.to_string())
            })
        })
        .next()?;

    // Normalizza soggetto: lowercase, trim, rimuovi articoli
    let subject = subject_part
        .trim()
        .to_lowercase()
        .trim_start_matches("the ")
        .trim_start_matches("a ")
        .trim_start_matches("an ")
        .to_string();

    if subject.len() < 3 {
        return None;
    }

    // Estrai primo numero dal rest
    let number = extract_first_number(&rest)?;

    Some((subject, number))
}

/// Estrae il primo numero da una stringa (supporta virgole e decimali).
fn extract_first_number(s: &str) -> Option<f64> {
    let mut num_str = String::new();
    let mut found_digit = false;

    for c in s.chars() {
        if c.is_ascii_digit() || (c == '.' && found_digit) {
            num_str.push(c);
            found_digit = true;
        } else if c == ',' && found_digit {
            // Skip comma in numbers like 12,742
            continue;
        } else if found_digit {
            break;
        }
    }

    if num_str.is_empty() {
        return None;
    }

    num_str.parse().ok()
}

/// Calcola il tier di un TrustRecord.
pub fn compute_tier(record: &TrustRecord, now_us: i64) -> VeritTier {
    let score = record.authority_score();
    let domains = record.distinct_domains();
    let high = record.high_authority_count();
    let age = record.age_days(now_us);
    let contradictions = record.contradictions.len();

    // T4: peso >= 50, 2+ institutional/peer-reviewed, eta > 7 giorni, 0 contraddizioni
    if score >= 50.0 && high >= 2 && age > 7.0 && contradictions == 0 {
        return VeritTier::T4;
    }

    // T3: peso >= 15, almeno 1 institutional o peer-reviewed
    if score >= 15.0 && high >= 1 {
        return VeritTier::T3;
    }

    // T2: 2+ attestazioni da domini diversi, peso >= 5
    if domains >= 2 && score >= 5.0 && record.attestations.len() >= 2 {
        return VeritTier::T2;
    }

    // T1: almeno 1 attestazione, peso >= 1
    if !record.attestations.is_empty() && score >= 1.0 {
        return VeritTier::T1;
    }

    VeritTier::T0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> i64 {
        chrono::Utc::now().timestamp_micros()
    }

    fn attest(domain: &str, tier: SourceTier) -> Attestation {
        Attestation {
            source_pubkey: "test".into(),
            domain: domain.into(),
            source_tier: tier,
            timestamp_us: now(),
        }
    }

    #[test]
    fn test_t0_empty() {
        let r = TrustRecord::new(now());
        assert_eq!(compute_tier(&r, now()), VeritTier::T0);
    }

    #[test]
    fn test_t0_low_weight() {
        let mut r = TrustRecord::new(now());
        r.attestations.push(attest("test", SourceTier::Anonymous));
        // Anonymous weight = 0.5, need >= 1.0 for T1
        assert_eq!(compute_tier(&r, now()), VeritTier::T0);
    }

    #[test]
    fn test_t1_single_website() {
        let mut r = TrustRecord::new(now());
        r.attestations.push(attest("science", SourceTier::Website));
        assert_eq!(compute_tier(&r, now()), VeritTier::T1);
    }

    #[test]
    fn test_t1_single_peer_reviewed() {
        let mut r = TrustRecord::new(now());
        r.attestations.push(attest("science", SourceTier::PeerReviewed));
        // score=5, but only 1 domain, so T1 not T2
        assert_eq!(compute_tier(&r, now()), VeritTier::T1);
    }

    #[test]
    fn test_t2_multi_domain() {
        let mut r = TrustRecord::new(now());
        r.attestations.push(attest("science", SourceTier::PeerReviewed));
        r.attestations.push(attest("geography", SourceTier::Website));
        // score=6, 2 domains, 2 attestations
        assert_eq!(compute_tier(&r, now()), VeritTier::T2);
    }

    #[test]
    fn test_t2_not_enough_domains() {
        let mut r = TrustRecord::new(now());
        r.attestations.push(attest("science", SourceTier::PeerReviewed));
        r.attestations.push(attest("science", SourceTier::Website));
        // score=6, but only 1 domain -> T1
        assert_eq!(compute_tier(&r, now()), VeritTier::T1);
    }

    #[test]
    fn test_t3_institutional() {
        let mut r = TrustRecord::new(now());
        r.attestations.push(attest("science", SourceTier::Institutional));
        r.attestations.push(attest("geography", SourceTier::PeerReviewed));
        // score=15, high=2
        assert_eq!(compute_tier(&r, now()), VeritTier::T3);
    }

    #[test]
    fn test_t3_just_enough() {
        let mut r = TrustRecord::new(now());
        r.attestations.push(attest("science", SourceTier::Institutional));
        r.attestations.push(attest("geo", SourceTier::PeerReviewed));
        // score=15, high=2, but T4 needs score>=50
        assert_eq!(compute_tier(&r, now()), VeritTier::T3);
    }

    #[test]
    fn test_t4_full() {
        let old = now() - 8 * 86400 * 1_000_000; // 8 days ago
        let mut r = TrustRecord::new(old);
        // 5 institutional = 50 weight
        for i in 0..5 {
            r.attestations.push(attest(&format!("d{i}"), SourceTier::Institutional));
        }
        assert_eq!(compute_tier(&r, now()), VeritTier::T4);
    }

    #[test]
    fn test_t4_blocked_by_contradiction() {
        let old = now() - 8 * 86400 * 1_000_000;
        let mut r = TrustRecord::new(old);
        for i in 0..5 {
            r.attestations.push(attest(&format!("d{i}"), SourceTier::Institutional));
        }
        r.contradictions.push(Contradiction {
            conflicting_fact_id: "x".into(),
            description: "conflict".into(),
            source_pubkey: "y".into(),
            timestamp_us: now(),
        });
        // Has contradiction -> cannot be T4
        assert_eq!(compute_tier(&r, now()), VeritTier::T3);
    }

    #[test]
    fn test_t4_blocked_by_age() {
        let recent = now() - 1 * 86400 * 1_000_000; // 1 day ago
        let mut r = TrustRecord::new(recent);
        for i in 0..5 {
            r.attestations.push(attest(&format!("d{i}"), SourceTier::Institutional));
        }
        // Too young for T4 (need >7 days)
        assert_eq!(compute_tier(&r, now()), VeritTier::T3);
    }

    #[test]
    fn test_authority_score() {
        let mut r = TrustRecord::new(now());
        r.attestations.push(attest("a", SourceTier::Institutional));
        r.attestations.push(attest("b", SourceTier::PeerReviewed));
        r.attestations.push(attest("c", SourceTier::Anonymous));
        assert!((r.authority_score() - 15.5).abs() < 0.01);
    }

    #[test]
    fn test_source_tier_from_str() {
        assert_eq!(SourceTier::from_str_loose("institutional"), SourceTier::Institutional);
        assert_eq!(SourceTier::from_str_loose("peer_reviewed"), SourceTier::PeerReviewed);
        assert_eq!(SourceTier::from_str_loose("media"), SourceTier::MainstreamMedia);
        assert_eq!(SourceTier::from_str_loose("garbage"), SourceTier::Anonymous);
    }

    #[test]
    fn test_display() {
        assert_eq!(VeritTier::T0.to_string(), "T0");
        assert_eq!(VeritTier::T4.to_string(), "T4");
    }

    #[test]
    fn test_query_count_starts_zero() {
        let r = TrustRecord::new(now());
        assert_eq!(r.query_count, 0);
    }

    // === Independence tests ===

    #[test]
    fn test_independence_empty() {
        assert!((compute_independence(&[]) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_independence_single() {
        let a = vec![attest("science", SourceTier::PeerReviewed)];
        assert!((compute_independence(&a) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_independence_same_domain() {
        let a = vec![
            attest("science", SourceTier::PeerReviewed),
            attest("science", SourceTier::Website),
        ];
        // 1.0 + 0.3 = 1.3
        assert!((compute_independence(&a) - 1.3).abs() < 0.01);
    }

    #[test]
    fn test_independence_different_domains() {
        let a = vec![
            attest("science", SourceTier::PeerReviewed),
            attest("geography", SourceTier::Website),
        ];
        // Both fully independent: 2.0
        assert!((compute_independence(&a) - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_independence_mixed() {
        let a = vec![
            attest("science", SourceTier::Institutional),
            attest("science", SourceTier::PeerReviewed),
            attest("geography", SourceTier::Website),
        ];
        // science group: 1.0 + 0.3 = 1.3, geography: 1.0, total: 2.3
        assert!((compute_independence(&a) - 2.3).abs() < 0.01);
    }

    // === Contradiction detection tests ===

    #[test]
    fn test_extract_subject_number_basic() {
        let (subj, num) = extract_subject_number("Earth has a diameter of 12742 km").unwrap();
        assert_eq!(subj, "earth");
        assert!((num - 12742.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_subject_number_is() {
        let (subj, num) = extract_subject_number("The speed of light is 299792458 m/s").unwrap();
        assert!(subj.contains("speed of light"));
        assert!((num - 299792458.0).abs() < 1.0);
    }

    #[test]
    fn test_extract_subject_number_with_comma() {
        let (_, num) = extract_subject_number("France has a population of 67,750,000").unwrap();
        assert!((num - 67750000.0).abs() < 1.0);
    }

    #[test]
    fn test_extract_subject_number_none() {
        assert!(extract_subject_number("Hello world").is_none());
        assert!(extract_subject_number("The sky is blue").is_none());
    }

    #[test]
    fn test_extract_subject_number_the_prefix() {
        let (subj, _) = extract_subject_number("The Moon is 384400 km from Earth").unwrap();
        assert_eq!(subj, "moon");
    }

    // === Source breakdown tests ===

    #[test]
    fn test_source_breakdown() {
        let mut r = TrustRecord::new(now());
        r.attestations.push(attest("a", SourceTier::Institutional));
        r.attestations.push(attest("b", SourceTier::PeerReviewed));
        r.attestations.push(attest("c", SourceTier::PeerReviewed));
        r.attestations.push(attest("d", SourceTier::Website));
        let b = r.source_breakdown();
        assert_eq!(b.institutional, 1);
        assert_eq!(b.peer_reviewed, 2);
        assert_eq!(b.website, 1);
        assert_eq!(b.anonymous, 0);
    }

    // === Tier label tests ===

    #[test]
    fn test_tier_labels() {
        assert_eq!(tier_label(&VeritTier::T0), "Unattested");
        assert_eq!(tier_label(&VeritTier::T1), "Attested");
        assert_eq!(tier_label(&VeritTier::T2), "Corroborated");
        assert_eq!(tier_label(&VeritTier::T3), "Authoritative");
        assert_eq!(tier_label(&VeritTier::T4), "Canonical");
    }
}
