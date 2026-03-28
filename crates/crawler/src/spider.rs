//! Semantic Spider — autonomous web-of-knowledge crawler.
//!
//! Crawls topics from Wikipedia REST API + Wikidata SPARQL + REST Countries.
//! Extracts facts, discovers new topics, feeds into the VARCAVIA verification pipeline.
//! Best-effort: all network errors are caught and skipped.

use crate::ExtractedFact;
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::Mutex;

const USER_AGENT: &str = "VARCAVIA/0.1 (https://varcavia.com; verification-protocol)";

/// 28 starting seeds across knowledge domains.
pub const INITIAL_SEEDS: &[(&str, &str)] = &[
    ("Earth", "geography"), ("Continents", "geography"),
    ("Oceans", "geography"), ("Countries of the world", "geography"),
    ("Speed of light", "science"), ("Periodic table", "science"),
    ("Solar system", "science"), ("DNA", "science"),
    ("Gravity", "science"), ("Photosynthesis", "science"), ("Atom", "science"),
    ("World War II", "history"), ("Roman Empire", "history"),
    ("Industrial Revolution", "history"), ("Ancient Egypt", "history"),
    ("Internet", "technology"), ("Computer", "technology"), ("Electricity", "technology"),
    ("Human body", "health"), ("Vaccines", "health"), ("Heart", "health"),
    ("Climate change", "climate"), ("Carbon dioxide", "climate"),
    ("United Nations", "politics"), ("European Union", "politics"),
    ("Pi", "science"), ("Prime number", "science"), ("Water", "science"),
];

pub struct Seed {
    pub topic: String,
    pub depth: u32,
    pub domain: String,
}

pub struct SpiderStats {
    pub topics_crawled: AtomicU64,
    pub facts_discovered: AtomicU64,
    pub facts_new: AtomicU64,
    pub facts_attested: AtomicU64,
    pub contradictions_found: AtomicU64,
}

impl Default for SpiderStats {
    fn default() -> Self {
        Self {
            topics_crawled: AtomicU64::new(0),
            facts_discovered: AtomicU64::new(0),
            facts_new: AtomicU64::new(0),
            facts_attested: AtomicU64::new(0),
            contradictions_found: AtomicU64::new(0),
        }
    }
}

pub struct SemanticSpider {
    pending: Arc<Mutex<VecDeque<Seed>>>,
    visited: Arc<Mutex<HashSet<String>>>,
    pub stats: Arc<SpiderStats>,
}

impl Default for SemanticSpider {
    fn default() -> Self { Self::new() }
}

impl SemanticSpider {
    pub fn new() -> Self {
        let mut q = VecDeque::new();
        for (topic, domain) in INITIAL_SEEDS {
            q.push_back(Seed {
                topic: topic.to_string(),
                depth: 0,
                domain: domain.to_string(),
            });
        }
        Self {
            pending: Arc::new(Mutex::new(q)),
            visited: Arc::new(Mutex::new(HashSet::new())),
            stats: Arc::new(SpiderStats::default()),
        }
    }

    pub async fn next_seed(&self) -> Option<Seed> {
        self.pending.lock().await.pop_front()
    }

    pub async fn add_seed(&self, seed: Seed) {
        self.pending.lock().await.push_back(seed);
    }

    pub async fn is_visited(&self, topic: &str) -> bool {
        self.visited.lock().await.contains(&topic.to_lowercase())
    }

    pub async fn mark_visited(&self, topic: &str) {
        self.visited.lock().await.insert(topic.to_lowercase());
    }

    pub async fn queue_size(&self) -> usize {
        self.pending.lock().await.len()
    }

    pub async fn reseed(&self) {
        let mut q = self.pending.lock().await;
        for (topic, domain) in INITIAL_SEEDS {
            q.push_back(Seed {
                topic: topic.to_string(),
                depth: 0,
                domain: domain.to_string(),
            });
        }
    }
}

pub struct CrawlResult {
    pub facts: Vec<ExtractedFact>,
    pub new_seeds: Vec<(String, String)>, // (topic, domain)
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(USER_AGENT)
        .build()
        .unwrap_or_default()
}

/// Crawl a single topic from Wikipedia REST API.
pub async fn crawl_topic(topic: &str, domain: &str) -> CrawlResult {
    let mut facts = Vec::new();
    let mut new_seeds = Vec::new();

    // 1. Wikipedia REST API summary
    if let Some((wiki_facts, wiki_seeds)) = crawl_wikipedia_rest(topic, domain).await {
        facts.extend(wiki_facts);
        new_seeds.extend(wiki_seeds);
    }

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // 2. Wikidata properties
    if let Some(wd_facts) = crawl_wikidata_properties(topic, domain).await {
        facts.extend(wd_facts);
    }

    // Dedup within this batch
    let mut seen = HashSet::new();
    facts.retain(|f| seen.insert(f.text.clone()));

    CrawlResult { facts, new_seeds }
}

/// Wikipedia REST API summary extraction.
async fn crawl_wikipedia_rest(topic: &str, domain: &str) -> Option<(Vec<ExtractedFact>, Vec<(String, String)>)> {
    let title = topic.replace(' ', "_");
    let url = format!("https://en.wikipedia.org/api/rest_v1/page/summary/{title}");

    let resp = client().get(&url).send().await.ok()?;
    if !resp.status().is_success() { return None; }
    let data: serde_json::Value = resp.json().await.ok()?;

    let extract = data.get("extract")?.as_str()?;
    let source = format!("wikipedia:{title}");
    let mut facts = Vec::new();

    // Extract factual sentences
    for sentence in extract.split(". ") {
        let s = sentence.trim();
        if s.len() < 20 || s.len() > 300 { continue; }

        let lower = s.to_lowercase();
        let has_number = s.chars().any(|c| c.is_ascii_digit());
        let has_pattern = lower.contains(" is ") || lower.contains(" was ")
            || lower.contains(" has ") || lower.contains(" are ")
            || lower.contains(" were ") || lower.contains(" contains ")
            || lower.contains(" founded ") || lower.contains(" discovered ");

        if has_pattern || has_number {
            let text = if s.ends_with('.') { s.to_string() } else { format!("{s}.") };
            facts.push(ExtractedFact {
                text,
                domain: domain.to_string(),
                source: source.clone(),
            });
        }

        if facts.len() >= 10 { break; }
    }

    // Extract new seeds from "See also"-style proper nouns
    let new_seeds: Vec<(String, String)> = extract_concepts(extract)
        .into_iter()
        .take(10)
        .map(|c| (c, domain.to_string()))
        .collect();

    Some((facts, new_seeds))
}

/// Wikidata SPARQL: get properties of a topic.
async fn crawl_wikidata_properties(topic: &str, domain: &str) -> Option<Vec<ExtractedFact>> {
    let query = format!(
        r#"SELECT ?propLabel ?valLabel WHERE {{
  ?item rdfs:label "{topic}"@en .
  ?item ?p ?statement .
  ?statement ?ps ?val .
  ?prop wikibase:claim ?p ; wikibase:statementProperty ?ps .
  FILTER(LANG(?valLabel) = "en" || DATATYPE(?val) IN (xsd:decimal, xsd:integer, xsd:dateTime))
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language "en" }}
}} LIMIT 50"#
    );

    let resp = client()
        .get("https://query.wikidata.org/sparql")
        .query(&[("query", &query), ("format", &"json".to_string())])
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() { return None; }
    let data: serde_json::Value = resp.json().await.ok()?;

    let bindings = data.get("results")?.get("bindings")?.as_array()?;
    let mut facts = Vec::new();

    // Properties to skip (technical metadata, not factual)
    let skip_props: HashSet<&str> = [
        "height", "width", "duration", "aspect ratio", "number of pages",
        "Commons category", "Commons gallery", "image", "Wikimedia",
        "template", "taxon", "Unicode", "topic's main category",
        "category", "icon", "logo", "flag", "coat of arms", "page banner",
        "pronunciation", "IPA", "described by source", "maintained by",
        "instance of", "subclass of", "facet of", "topic's main template",
        "Stack Exchange tag", "Freebase ID", "GND ID", "VIAF ID",
        "Library of Congress", "ISNI", "BnF ID", "NKC ID",
        "subreddit", "hashtag", "GitHub", "Twitter", "Facebook",
    ].into_iter().collect();

    for row in bindings {
        let prop = row.get("propLabel")?.get("value")?.as_str()?;
        let val = row.get("valLabel")?.get("value")?.as_str()?;

        // Skip internal/technical properties
        if prop.starts_with('P') || val.starts_with('Q') || val.starts_with("http") {
            continue;
        }
        if prop.len() < 3 || val.len() < 2 { continue; }

        // Skip known noise properties
        if skip_props.iter().any(|s| prop.to_lowercase().contains(&s.to_lowercase())) {
            continue;
        }

        // Skip bare numbers without context (e.g., "21", "210")
        if val.parse::<f64>().is_ok() && val.len() < 6 {
            continue;
        }

        // Format dates: "1969-10-29T00:00:00Z" → "1969"
        let formatted_val = if val.contains('T') && val.contains('-') && val.len() > 10 {
            val.get(..4).unwrap_or(val).to_string()
        } else {
            val.to_string()
        };

        let text = format!("The {prop} of {topic} is {formatted_val}.");
        if text.len() > 20 && text.len() < 300 {
            facts.push(ExtractedFact {
                text,
                domain: domain.to_string(),
                source: format!("wikidata:{topic}"),
            });
        }

        if facts.len() >= 15 { break; }
    }

    Some(facts)
}

/// Extract proper noun concepts from text to use as new seeds.
pub fn extract_concepts(text: &str) -> Vec<String> {
    let skip: HashSet<&str> = [
        "The", "A", "An", "In", "On", "At", "By", "For", "And", "Or", "But",
        "With", "From", "To", "Of", "Is", "Was", "Are", "Were", "Has", "Had",
        "Its", "Their", "This", "That", "It", "He", "She", "They", "Which",
        "Who", "When", "Where", "What", "How", "Not", "Also", "However",
        "Although", "Because", "Since", "During", "After", "Before",
    ].into_iter().collect();

    let mut concepts = Vec::new();
    let mut seen = HashSet::new();

    for sentence in text.split(['.', ',', ';']) {
        let words: Vec<&str> = sentence.split_whitespace().collect();
        let mut i = 1; // Skip first word (sentence-initial cap)

        while i < words.len() {
            let clean = words[i].trim_matches(|c: char| !c.is_alphanumeric());
            if clean.chars().next().is_some_and(|c| c.is_uppercase()) && !skip.contains(clean) {
                let mut phrase = vec![clean.to_string()];
                let mut j = i + 1;
                while j < words.len() {
                    let next = words[j].trim_matches(|c: char| !c.is_alphanumeric());
                    if next.chars().next().is_some_and(|c| c.is_uppercase()) && !skip.contains(next) {
                        phrase.push(next.to_string());
                        j += 1;
                    } else {
                        break;
                    }
                }
                let concept = phrase.join(" ");
                if concept.len() >= 3 && seen.insert(concept.clone()) {
                    concepts.push(concept);
                }
                i = j;
            } else {
                i += 1;
            }
        }
    }

    concepts
}

/// Detect if two facts about the same subject AND attribute have contradicting numbers.
/// Requires high keyword overlap (>=0.6) to avoid false positives like
/// "Earth radius 6371" vs "Earth population 8 billion".
pub fn detect_contradiction(existing: &str, new_fact: &str) -> Option<ContradictionInfo> {
    let ex_kw = varcavia_uag::keyword_match::extract_keywords(existing);
    let new_kw = varcavia_uag::keyword_match::extract_keywords(new_fact);
    let overlap = varcavia_uag::keyword_match::keyword_overlap(&ex_kw, &new_kw);

    // Require high overlap — same subject AND same attribute
    if overlap < 0.6 { return None; }

    let ex_nums = varcavia_uag::keyword_match::extract_numbers(existing);
    let new_nums = varcavia_uag::keyword_match::extract_numbers(new_fact);

    // Need exactly 1 number each for a meaningful comparison
    if ex_nums.len() != 1 || new_nums.len() != 1 { return None; }

    let en = ex_nums[0];
    let nn = new_nums[0];
    if en == 0.0 || nn == 0.0 { return None; }

    let max_val = en.abs().max(nn.abs());
    let divergence = (en - nn).abs() / max_val;

    // Only flag if numbers are close enough to be about the same thing
    // but different enough to be a real disagreement (1% to 50%)
    if divergence > 0.01 && divergence < 0.5 {
        return Some(ContradictionInfo {
            divergence_pct: divergence * 100.0,
            existing_num: en,
            new_num: nn,
        });
    }
    None
}

pub struct ContradictionInfo {
    pub divergence_pct: f64,
    pub existing_num: f64,
    pub new_num: f64,
}

/// Extract the main subject from a fact sentence.
pub fn extract_subject(text: &str) -> String {
    let delimiters = [" is ", " has ", " was ", " are ", " were ", " covers ", " measures ", " contains "];
    for d in &delimiters {
        if let Some(pos) = text.to_lowercase().find(d) {
            let subject = &text[..pos];
            let clean = subject
                .trim_start_matches("The ")
                .trim_start_matches("A ")
                .trim_start_matches("An ")
                .trim();
            if !clean.is_empty() {
                return clean.to_string();
            }
        }
    }
    text.split_whitespace().take(3).collect::<Vec<_>>().join(" ")
}

/// Simple consistency checks for numerical facts.
pub fn check_consistency(fact: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let lower = fact.to_lowercase();
    let nums = varcavia_uag::keyword_match::extract_numbers(fact);

    for num in &nums {
        if lower.contains("population") && *num < 0.0 {
            issues.push("Population cannot be negative".into());
        }
        if lower.contains("area") && *num <= 0.0 {
            issues.push("Area cannot be zero or negative".into());
        }
        if lower.contains("temperature") && lower.contains("celsius") && *num < -273.15 {
            issues.push("Temperature below absolute zero".into());
        }
        if lower.contains("speed") && *num > 299_792_458.0 && !lower.contains("light") {
            issues.push("Speed exceeds speed of light".into());
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_concepts_basic() {
        let concepts = extract_concepts("DNA was discovered by Watson and Crick in 1953");
        assert!(concepts.iter().any(|c| c.contains("Watson")));
        assert!(concepts.iter().any(|c| c.contains("Crick")));
    }

    #[test]
    fn test_extract_concepts_skips_articles() {
        let concepts = extract_concepts("The United Nations was founded in San Francisco");
        assert!(concepts.iter().any(|c| c.contains("United Nations")));
        assert!(concepts.iter().any(|c| c.contains("San Francisco")));
    }

    #[test]
    fn test_extract_concepts_empty() {
        let concepts = extract_concepts("this has no proper nouns at all");
        assert!(concepts.is_empty());
    }

    #[test]
    fn test_extract_subject_basic() {
        assert_eq!(extract_subject("Nigeria has a population of 223 million"), "Nigeria");
    }

    #[test]
    fn test_extract_subject_the_prefix() {
        assert_eq!(extract_subject("The speed of light is 299792458 m/s"), "speed of light");
    }

    #[test]
    fn test_extract_subject_fallback() {
        assert_eq!(extract_subject("Hello world today"), "Hello world today");
    }

    #[test]
    fn test_detect_contradiction_found() {
        let c = detect_contradiction(
            "Nigeria has a population of 223 million",
            "Nigeria has a population of 190 million",
        );
        assert!(c.is_some());
        let info = c.unwrap();
        assert!(info.divergence_pct > 10.0); // ~17%
    }

    #[test]
    fn test_detect_contradiction_no_disagreement() {
        let c = detect_contradiction(
            "Earth has a radius of 6371 km",
            "Earth has a radius of 6371 kilometres",
        );
        assert!(c.is_none()); // Same number
    }

    #[test]
    fn test_detect_contradiction_different_attribute() {
        // Same subject but different attribute — NOT a contradiction
        let c = detect_contradiction(
            "Earth has a radius of 6371 km",
            "Earth has a population of 8 billion",
        );
        assert!(c.is_none()); // Low keyword overlap (radius vs population)
    }

    #[test]
    fn test_detect_contradiction_unrelated() {
        let c = detect_contradiction(
            "Gold has a melting point of 1064 degrees",
            "Brazil has a population of 214 million",
        );
        assert!(c.is_none()); // Low keyword overlap
    }

    #[test]
    fn test_consistency_ok_population() {
        // Positive population should have no issues
        let issues = check_consistency("Nigeria has a population of 223 million");
        assert!(issues.is_empty());
    }

    #[test]
    fn test_consistency_ok_area() {
        let issues = check_consistency("Brazil covers an area of 8515767 square km");
        assert!(issues.is_empty());
    }

    #[test]
    fn test_consistency_ok() {
        let issues = check_consistency("Earth has a population of 8 billion");
        assert!(issues.is_empty());
    }

    #[test]
    fn test_spider_creation() {
        let spider = SemanticSpider::new();
        assert_eq!(spider.stats.topics_crawled.load(std::sync::atomic::Ordering::Relaxed), 0);
    }
}
