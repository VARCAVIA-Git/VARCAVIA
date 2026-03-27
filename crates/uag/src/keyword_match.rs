//! Keyword extraction and number-aware matching for fuzzy fact verification.
//!
//! Provides semantic matching that works when the same fact is worded differently:
//! - "Earth has a diameter of 12742 km" matches "The diameter of Earth is 12,742 kilometres"
//! - "speed of light is 299792458 m/s" matches "the speed of light is 299,792,458 metres per second"
//!
//! Pure Rust, zero external dependencies.

use std::collections::HashSet;

const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "shall", "can", "need", "dare", "ought",
    "of", "in", "to", "for", "with", "on", "at", "from", "by", "about",
    "as", "into", "through", "during", "before", "after", "above", "below",
    "between", "under", "over", "up", "down", "out", "off", "then", "than",
    "and", "but", "or", "nor", "not", "so", "yet", "both", "either",
    "neither", "each", "every", "all", "any", "few", "more", "most",
    "other", "some", "such", "no", "only", "same", "that", "this",
    "it", "its", "also", "very", "often", "just", "approximately",
    "roughly", "about", "around", "nearly", "estimated", "known",
    "called", "named", "considered", "located", "found",
];

/// Unit normalization table: (variants, canonical).
/// IMPORTANT: multi-word patterns MUST come before their sub-patterns
/// (e.g. "metres per second" before "metres") to avoid partial replacement.
const UNIT_NORMS: &[(&[&str], &str)] = &[
    // Multi-word first
    (&["meters per second", "metres per second"], "m/s"),
    (&["square kilometres", "square kilometers", "sq km", "km2"], "sq_km"),
    (&["degrees celsius", "degrees c"], "celsius"),
    (&["degrees fahrenheit"], "fahrenheit"),
    (&["degrees kelvin"], "kelvin"),
    (&["per cent"], "percent"),
    // Then single-word
    (&["kilometres", "kilometers", "kilometre", "kilometer"], "km"),
    (&["metres", "meters", "metre", "meter"], "m"),
    (&["celsius", "°c"], "celsius"),
    (&["fahrenheit", "°f"], "fahrenheit"),
    (&["kelvin"], "kelvin"),
    (&["kilograms", "kilogrammes", "kilogram"], "kg"),
    (&["miles"], "mi"),
    (&["feet"], "ft"),
    (&["pounds", "lbs"], "lb"),
    (&["litres", "liters", "litre", "liter"], "l"),
    (&["percent"], "percent"),
];

/// Extract content words from a fact string.
/// Removes articles, prepositions, common verbs. Normalizes to lowercase.
/// Numbers are excluded (handled separately by extract_numbers).
pub fn extract_keywords(text: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOP_WORDS.iter().copied().collect();
    let normalized = normalize_units(text);

    normalized
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|w| w.to_lowercase())
        .filter(|w| {
            w.len() >= 2
                && !stop.contains(w.as_str())
                && !w.chars().all(|c| c.is_ascii_digit())
        })
        .collect()
}

/// Extract all numbers from text, normalizing commas and written multipliers.
pub fn extract_numbers(text: &str) -> Vec<f64> {
    let mut numbers = Vec::new();
    let lower = text.to_lowercase();

    // First pass: find digit sequences (with commas and decimals)
    let mut i = 0;
    let chars: Vec<char> = lower.chars().collect();
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            let mut num_str = String::new();
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == ',') {
                if chars[i] != ',' {
                    num_str.push(chars[i]);
                }
                i += 1;
            }

            if let Ok(n) = num_str.parse::<f64>() {
                // Check for trailing multiplier word
                let rest: String = chars[i..].iter().collect();
                let rest = rest.trim_start();
                let multiplied = if rest.starts_with("billion") {
                    n * 1_000_000_000.0
                } else if rest.starts_with("million") {
                    n * 1_000_000.0
                } else if rest.starts_with("trillion") {
                    n * 1_000_000_000_000.0
                } else if rest.starts_with("thousand") {
                    n * 1_000.0
                } else {
                    n
                };
                numbers.push(multiplied);
            }
            // Check if we consumed enough from the original text
            let _ = start; // suppress unused warning
        } else {
            i += 1;
        }
    }

    numbers
}

/// Normalize unit variations to canonical form.
/// Only replaces whole words (bounded by non-alpha chars or string edges).
pub fn normalize_units(text: &str) -> String {
    let mut result = text.to_lowercase();
    for (variants, canonical) in UNIT_NORMS {
        for variant in *variants {
            // Only replace if it appears as a whole word (not inside another word)
            let needle = *variant;
            let mut search_from = 0;
            while let Some(pos) = result[search_from..].find(needle) {
                let abs_pos = search_from + pos;
                let end_pos = abs_pos + needle.len();
                let before_ok = abs_pos == 0
                    || !result.as_bytes()[abs_pos - 1].is_ascii_alphabetic();
                let after_ok = end_pos >= result.len()
                    || !result.as_bytes()[end_pos].is_ascii_alphabetic();
                if before_ok && after_ok {
                    result = format!("{}{}{}", &result[..abs_pos], canonical, &result[end_pos..]);
                    search_from = abs_pos + canonical.len();
                } else {
                    search_from = abs_pos + 1;
                }
            }
        }
    }
    result
}

/// Compute keyword overlap ratio (Jaccard similarity).
pub fn keyword_overlap(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let set_a: HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

/// Check if two sets of numbers match within tolerance.
/// Every number in the smaller set must have a match in the larger set.
/// Returns true if both sets are empty.
pub fn numbers_match(a: &[f64], b: &[f64], tolerance: f64) -> bool {
    if a.is_empty() && b.is_empty() {
        return true;
    }
    if a.is_empty() || b.is_empty() {
        // One has numbers, other doesn't — not a mismatch, just missing info
        return true;
    }

    let (smaller, larger) = if a.len() <= b.len() { (a, b) } else { (b, a) };

    for &n in smaller {
        let matched = larger.iter().any(|&m| {
            if n == 0.0 && m == 0.0 {
                return true;
            }
            let max_val = n.abs().max(m.abs());
            if max_val == 0.0 {
                return true;
            }
            (n - m).abs() / max_val <= tolerance
        });
        if !matched {
            return false;
        }
    }

    true
}

/// Full keyword-based matching score.
/// Returns (score, nums_match) where score is 0.0..1.0.
pub fn keyword_match_score(query: &str, candidate: &str) -> (f64, bool) {
    let kw_a = extract_keywords(query);
    let kw_b = extract_keywords(candidate);
    let overlap = keyword_overlap(&kw_a, &kw_b);

    let nums_a = extract_numbers(query);
    let nums_b = extract_numbers(candidate);
    let nums_ok = numbers_match(&nums_a, &nums_b, 0.01);

    (overlap, nums_ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Keyword extraction ===

    #[test]
    fn test_extract_keywords_strips_stop_words() {
        let kw = extract_keywords("The diameter of Earth is 12,742 kilometres");
        assert!(kw.contains(&"diameter".to_string()));
        assert!(kw.contains(&"earth".to_string()));
        assert!(!kw.contains(&"the".to_string()));
        assert!(!kw.contains(&"of".to_string()));
        assert!(!kw.contains(&"is".to_string()));
    }

    #[test]
    fn test_extract_keywords_lowercase() {
        let kw = extract_keywords("NASA confirmed the discovery");
        assert!(kw.contains(&"nasa".to_string()));
        assert!(kw.contains(&"confirmed".to_string()));
        assert!(kw.contains(&"discovery".to_string()));
    }

    // === Number extraction ===

    #[test]
    fn test_extract_numbers_with_commas() {
        let nums = extract_numbers("299,792,458 m/s");
        assert_eq!(nums.len(), 1);
        assert!((nums[0] - 299792458.0).abs() < 1.0);
    }

    #[test]
    fn test_extract_numbers_decimal() {
        let nums = extract_numbers("approximately 3.14159");
        assert_eq!(nums.len(), 1);
        assert!((nums[0] - 3.14159).abs() < 0.001);
    }

    #[test]
    fn test_extract_numbers_written_million() {
        let nums = extract_numbers("14 million people");
        assert_eq!(nums.len(), 1);
        assert!((nums[0] - 14_000_000.0).abs() < 1.0);
    }

    #[test]
    fn test_extract_numbers_written_billion() {
        let nums = extract_numbers("7.9 billion");
        assert_eq!(nums.len(), 1);
        assert!((nums[0] - 7_900_000_000.0).abs() < 1000.0);
    }

    #[test]
    fn test_extract_numbers_multiple() {
        let nums = extract_numbers("from 1879 to 1955");
        assert_eq!(nums.len(), 2);
    }

    // === Unit normalization ===

    #[test]
    fn test_normalize_units_km() {
        assert!(normalize_units("12742 kilometres").contains("km"));
        assert!(normalize_units("12742 kilometers").contains("km"));
    }

    #[test]
    fn test_normalize_units_celsius() {
        assert!(normalize_units("100 degrees celsius").contains("celsius"));
        assert!(normalize_units("100 °C").contains("celsius"));
    }

    #[test]
    fn test_normalize_units_ms() {
        assert!(normalize_units("metres per second").contains("m/s"));
    }

    // === Number matching ===

    #[test]
    fn test_numbers_match_exact() {
        assert!(numbers_match(&[12742.0], &[12742.0], 0.01));
    }

    #[test]
    fn test_numbers_match_within_tolerance() {
        assert!(numbers_match(&[12742.0], &[12740.0], 0.01));
    }

    #[test]
    fn test_numbers_dont_match_outside_tolerance() {
        assert!(!numbers_match(&[14_000_000.0], &[37_000_000.0], 0.01));
    }

    #[test]
    fn test_numbers_match_both_empty() {
        assert!(numbers_match(&[], &[], 0.01));
    }

    #[test]
    fn test_numbers_match_one_empty() {
        // One side has no numbers — not a contradiction
        assert!(numbers_match(&[100.0], &[], 0.01));
    }

    // === Full keyword matching — CRITICAL TESTS ===

    #[test]
    fn test_keyword_match_diameter_of_earth() {
        let (score, nums) = keyword_match_score(
            "Earth has a diameter of 12742 km",
            "The diameter of Earth is 12,742 km",
        );
        assert!(score >= 0.6, "Expected overlap >= 0.6, got {score}");
        assert!(nums, "Numbers should match");
    }

    #[test]
    fn test_keyword_match_speed_of_light() {
        let (score, nums) = keyword_match_score(
            "speed of light is 299792458 m/s",
            "The speed of light in vacuum is exactly 299,792,458 m/s.",
        );
        assert!(score >= 0.4, "Expected overlap >= 0.4, got {score}");
        assert!(nums, "Numbers should match");
    }

    #[test]
    fn test_keyword_match_boiling_point() {
        let (score, nums) = keyword_match_score(
            "Water boils at 100 celsius",
            "Water boils at 100 celsius at standard atmospheric pressure.",
        );
        assert!(score >= 0.4, "Expected overlap >= 0.4, got {score}");
        assert!(nums, "Numbers should match");
    }

    #[test]
    fn test_keyword_no_match_different_numbers() {
        let (_, nums) = keyword_match_score(
            "Tokyo has a population of 14 million",
            "Tokyo has a population of 37 million",
        );
        assert!(!nums, "Numbers should NOT match (14M vs 37M)");
    }

    #[test]
    fn test_keyword_no_match_different_subject() {
        let (score, _) = keyword_match_score(
            "Earth is round",
            "Mars is round",
        );
        assert!(score < 0.5, "Different subjects should have low overlap, got {score}");
    }

    #[test]
    fn test_keyword_match_no_numbers() {
        let (score, nums) = keyword_match_score(
            "Earth is the third planet from the Sun",
            "The third planet from the Sun is Earth",
        );
        assert!(score >= 0.6, "Same keywords, different order, got {score}");
        assert!(nums, "No numbers on either side = match");
    }

    #[test]
    fn test_keyword_match_reworded_fact() {
        let (score, nums) = keyword_match_score(
            "The Moon orbits Earth every 27.3 days",
            "The Moon orbits Earth once every 27.3 days.",
        );
        assert!(score >= 0.5, "Expected >= 0.5, got {score}");
        assert!(nums);
    }
}
