//! Wikidata SPARQL crawler — estrae fatti strutturati da Wikidata.
//!
//! Best-effort: se la rete non e disponibile, restituisce vettore vuoto.
//! Rate limit: max 1 request/secondo.

use crate::ExtractedFact;

const SPARQL_ENDPOINT: &str = "https://query.wikidata.org/sparql";
const USER_AGENT: &str = "VARCAVIA/0.1 (https://github.com/VARCAVIA-Git/VARCAVIA; fact-verification-protocol)";

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(USER_AGENT)
        .build()
        .unwrap_or_default()
}

/// Esegue una query SPARQL e restituisce i risultati come JSON.
async fn sparql_query(query: &str) -> Option<serde_json::Value> {
    let resp = client()
        .get(SPARQL_ENDPOINT)
        .query(&[("query", query), ("format", "json")])
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        tracing::warn!("Wikidata SPARQL: HTTP {}", resp.status());
        return None;
    }

    resp.json().await.ok()
}

/// Estrae i bindings dai risultati SPARQL.
fn extract_bindings(data: &serde_json::Value) -> Vec<&serde_json::Value> {
    data.get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}

/// Helper: estrai stringa da un binding SPARQL.
fn binding_str<'a>(row: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    row.get(field)?.get("value")?.as_str()
}

/// Formatta un numero grande con suffisso (million, billion).
fn format_number(s: &str) -> String {
    if let Ok(n) = s.parse::<f64>() {
        if n >= 1_000_000_000.0 {
            format!("{:.2} billion", n / 1_000_000_000.0)
        } else if n >= 1_000_000.0 {
            format!("{:.2} million", n / 1_000_000.0)
        } else if n >= 1000.0 {
            format!("{}", n.round() as i64)
        } else {
            format!("{:.1}", n)
        }
    } else {
        s.to_string()
    }
}

// === SPARQL Queries ===

const COUNTRIES_QUERY: &str = r#"
SELECT ?countryLabel ?capitalLabel ?population ?area ?continentLabel WHERE {
  ?country wdt:P31 wd:Q6256.
  OPTIONAL { ?country wdt:P36 ?capital. }
  OPTIONAL { ?country wdt:P1082 ?population. }
  OPTIONAL { ?country wdt:P2046 ?area. }
  OPTIONAL { ?country wdt:P30 ?continent. }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "en". }
}
ORDER BY DESC(?population)
LIMIT 200
"#;

const ELEMENTS_QUERY: &str = r#"
SELECT ?elementLabel ?symbol ?atomicNumber WHERE {
  ?element wdt:P31 wd:Q11344.
  ?element wdt:P246 ?symbol.
  ?element wdt:P1086 ?atomicNumber.
  SERVICE wikibase:label { bd:serviceParam wikibase:language "en". }
}
ORDER BY ?atomicNumber
LIMIT 120
"#;

const PLANETS_QUERY: &str = r#"
SELECT ?planetLabel ?diameter ?distSun ?moons WHERE {
  ?planet wdt:P31 wd:Q634.
  OPTIONAL { ?planet wdt:P2386 ?diameter. }
  OPTIONAL { ?planet wdt:P2243 ?distSun. }
  OPTIONAL { ?planet wdt:P397 ?moons. }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "en". }
}
LIMIT 20
"#;

const PEOPLE_QUERY: &str = r#"
SELECT ?personLabel ?birthDate ?deathDate ?occupationLabel WHERE {
  ?person wdt:P31 wd:Q5.
  ?person wikibase:sitelinks ?sitelinks.
  OPTIONAL { ?person wdt:P569 ?birthDate. }
  OPTIONAL { ?person wdt:P570 ?deathDate. }
  OPTIONAL { ?person wdt:P106 ?occupation. }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "en". }
}
ORDER BY DESC(?sitelinks)
LIMIT 100
"#;

/// Crawla tutti i batch Wikidata. Best-effort: errori non bloccano.
pub async fn crawl_wikidata() -> Vec<ExtractedFact> {
    let mut all_facts = Vec::new();

    tracing::info!("Wikidata crawler: avvio...");

    // Batch 1: Paesi
    if let Some(facts) = crawl_countries().await {
        tracing::info!("Wikidata: {} fatti da paesi", facts.len());
        all_facts.extend(facts);
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Batch 2: Elementi chimici
    if let Some(facts) = crawl_elements().await {
        tracing::info!("Wikidata: {} fatti da elementi", facts.len());
        all_facts.extend(facts);
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Batch 3: Pianeti
    if let Some(facts) = crawl_planets().await {
        tracing::info!("Wikidata: {} fatti da pianeti", facts.len());
        all_facts.extend(facts);
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Batch 4: Persone
    if let Some(facts) = crawl_people().await {
        tracing::info!("Wikidata: {} fatti da persone", facts.len());
        all_facts.extend(facts);
    }

    // Dedup
    let mut seen = std::collections::HashSet::new();
    all_facts.retain(|f| seen.insert(f.text.clone()));

    tracing::info!("Wikidata crawler completato: {} fatti totali", all_facts.len());
    all_facts
}

async fn crawl_countries() -> Option<Vec<ExtractedFact>> {
    let data = sparql_query(COUNTRIES_QUERY).await?;
    let bindings = extract_bindings(&data);
    let mut facts = Vec::new();

    for row in bindings {
        let country = binding_str(row, "countryLabel")?;
        if country.starts_with('Q') || country.starts_with("http") {
            continue; // Skip unresolved labels
        }

        if let Some(capital) = binding_str(row, "capitalLabel") {
            if !capital.starts_with('Q') {
                facts.push(ExtractedFact {
                    text: format!("The capital of {country} is {capital}."),
                    domain: "geography".into(),
                    source: "wikidata:countries".into(),
                });
            }
        }

        if let Some(pop) = binding_str(row, "population") {
            let formatted = format_number(pop);
            facts.push(ExtractedFact {
                text: format!("{country} has a population of {formatted}."),
                domain: "geography".into(),
                source: "wikidata:countries".into(),
            });
        }

        if let Some(area) = binding_str(row, "area") {
            if let Ok(a) = area.parse::<f64>() {
                let formatted = format!("{}", a.round() as i64);
                facts.push(ExtractedFact {
                    text: format!("{country} has an area of {formatted} square kilometres."),
                    domain: "geography".into(),
                    source: "wikidata:countries".into(),
                });
            }
        }

        if let Some(continent) = binding_str(row, "continentLabel") {
            if !continent.starts_with('Q') {
                facts.push(ExtractedFact {
                    text: format!("{country} is located in {continent}."),
                    domain: "geography".into(),
                    source: "wikidata:countries".into(),
                });
            }
        }
    }

    Some(facts)
}

async fn crawl_elements() -> Option<Vec<ExtractedFact>> {
    let data = sparql_query(ELEMENTS_QUERY).await?;
    let bindings = extract_bindings(&data);
    let mut facts = Vec::new();

    for row in bindings {
        let name = binding_str(row, "elementLabel")?;
        if name.starts_with('Q') { continue; }
        let symbol = binding_str(row, "symbol")?;
        let number = binding_str(row, "atomicNumber")?;

        facts.push(ExtractedFact {
            text: format!("{name} is a chemical element with symbol {symbol} and atomic number {number}."),
            domain: "science".into(),
            source: "wikidata:elements".into(),
        });
    }

    Some(facts)
}

async fn crawl_planets() -> Option<Vec<ExtractedFact>> {
    let data = sparql_query(PLANETS_QUERY).await?;
    let bindings = extract_bindings(&data);
    let mut facts = Vec::new();

    for row in bindings {
        let name = binding_str(row, "planetLabel")?;
        if name.starts_with('Q') { continue; }

        if let Some(d) = binding_str(row, "diameter") {
            if let Ok(km) = d.parse::<f64>() {
                facts.push(ExtractedFact {
                    text: format!("{name} has a diameter of {:.0} kilometres.", km),
                    domain: "science".into(),
                    source: "wikidata:planets".into(),
                });
            }
        }

        if let Some(dist) = binding_str(row, "distSun") {
            if let Ok(km) = dist.parse::<f64>() {
                let au = km / 149_597_870.7;
                facts.push(ExtractedFact {
                    text: format!("{name} orbits the Sun at an average distance of {:.2} AU.", au),
                    domain: "science".into(),
                    source: "wikidata:planets".into(),
                });
            }
        }
    }

    Some(facts)
}

async fn crawl_people() -> Option<Vec<ExtractedFact>> {
    let data = sparql_query(PEOPLE_QUERY).await?;
    let bindings = extract_bindings(&data);
    let mut facts = Vec::new();
    let mut seen_people = std::collections::HashSet::new();

    for row in bindings {
        let name = binding_str(row, "personLabel")?;
        if name.starts_with('Q') || !seen_people.insert(name.to_string()) {
            continue;
        }

        let birth = binding_str(row, "birthDate").map(|d| &d[..4.min(d.len())]);
        let death = binding_str(row, "deathDate").map(|d| &d[..4.min(d.len())]);
        let occupation = binding_str(row, "occupationLabel")
            .filter(|o| !o.starts_with('Q'));

        match (birth, death, occupation) {
            (Some(b), Some(d), Some(occ)) => {
                facts.push(ExtractedFact {
                    text: format!("{name} was a {occ} who lived from {b} to {d}."),
                    domain: "general".into(),
                    source: "wikidata:people".into(),
                });
            }
            (Some(b), None, Some(occ)) => {
                facts.push(ExtractedFact {
                    text: format!("{name} is a {occ} born in {b}."),
                    domain: "general".into(),
                    source: "wikidata:people".into(),
                });
            }
            (Some(b), Some(d), None) => {
                facts.push(ExtractedFact {
                    text: format!("{name} lived from {b} to {d}."),
                    domain: "general".into(),
                    source: "wikidata:people".into(),
                });
            }
            _ => {}
        }
    }

    Some(facts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number_billion() {
        assert_eq!(format_number("7900000000"), "7.90 billion");
    }

    #[test]
    fn test_format_number_million() {
        assert_eq!(format_number("67750000"), "67.75 million");
    }

    #[test]
    fn test_format_number_thousand() {
        assert_eq!(format_number("12742"), "12742");
    }

    #[test]
    fn test_format_number_small() {
        assert_eq!(format_number("3.14"), "3.1");
    }

    #[test]
    fn test_format_number_invalid() {
        assert_eq!(format_number("abc"), "abc");
    }

    #[test]
    fn test_extract_bindings_empty() {
        let data = serde_json::json!({"results": {"bindings": []}});
        assert!(extract_bindings(&data).is_empty());
    }

    #[test]
    fn test_extract_bindings_valid() {
        let data = serde_json::json!({
            "results": {
                "bindings": [
                    {"name": {"type": "literal", "value": "France"}}
                ]
            }
        });
        let bindings = extract_bindings(&data);
        assert_eq!(bindings.len(), 1);
        assert_eq!(binding_str(bindings[0], "name"), Some("France"));
    }

    #[test]
    fn test_binding_str_missing() {
        let row = serde_json::json!({"name": {"value": "test"}});
        assert_eq!(binding_str(&row, "name"), Some("test"));
        assert_eq!(binding_str(&row, "missing"), None);
    }
}
