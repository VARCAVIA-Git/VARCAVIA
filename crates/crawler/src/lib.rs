//! # VARCAVIA Crawler
//!
//! Crawler minimale che scarica pagine Wikipedia e ne estrae fatti verificabili.
//! Usato per popolare il nodo con dati reali prima del lancio pubblico.

/// Pagine Wikipedia da scaricare per l'estrazione di fatti.
const WIKI_PAGES: &[(&str, &str)] = &[
    ("Earth", "science"),
    ("France", "geography"),
    ("United_Nations", "politics"),
    ("Speed_of_light", "science"),
    ("Water", "science"),
    ("Albert_Einstein", "science"),
    ("Moon", "science"),
    ("Sun", "science"),
    ("Human_body", "health"),
    ("DNA", "science"),
    ("Oxygen", "science"),
    ("Gold", "science"),
    ("Python_(programming_language)", "technology"),
    ("Internet", "technology"),
    ("Tokyo", "geography"),
];

/// Un fatto estratto con il suo dominio.
#[derive(Debug, Clone)]
pub struct ExtractedFact {
    pub text: String,
    pub domain: String,
    pub source: String,
}

/// Estrae fatti da testo HTML Wikipedia (parsing semplificato).
pub fn extract_facts_from_html(html: &str, domain: &str, page: &str) -> Vec<ExtractedFact> {
    let mut facts = Vec::new();
    let source = format!("wikipedia:{page}");

    // Rimuovi tag HTML per ottenere testo pulito
    let text = strip_html(html);

    for line in text.lines() {
        let line = line.trim();
        if line.len() < 20 || line.len() > 300 {
            continue;
        }

        // Salta linee non informative
        if line.starts_with('[')
            || line.starts_with('{')
            || line.starts_with("Retrieved")
            || line.starts_with("See also")
            || line.starts_with("References")
            || line.starts_with("External links")
            || line.contains("Wikipedia")
            || line.contains("citation needed")
            || line.contains("edit]")
        {
            continue;
        }

        // Cerca pattern fattuali
        let is_factual = line.contains(" is ")
            || line.contains(" are ")
            || line.contains(" was ")
            || line.contains(" has ")
            || line.contains(" have ")
            || line.contains(" measures ")
            || line.contains(" weighs ")
            || line.contains(" contains ")
            || contains_number_with_unit(line);

        if is_factual {
            // Prendi solo la prima frase
            let fact = first_sentence(line);
            if fact.len() >= 20 && !facts.iter().any(|f: &ExtractedFact| f.text == fact) {
                facts.push(ExtractedFact {
                    text: fact,
                    domain: domain.to_string(),
                    source: source.clone(),
                });
            }
        }

        if facts.len() >= 10 {
            break;
        }
    }

    facts
}

/// Rimuove tag HTML in modo semplice.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    let lower = html.to_lowercase();
    let chars: Vec<char> = html.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        if !in_tag && chars[i] == '<' {
            in_tag = true;
            // Check for script/style start
            let rest: String = lower_chars[i..].iter().take(20).collect();
            if rest.starts_with("<script") {
                in_script = true;
            } else if rest.starts_with("<style") {
                in_style = true;
            } else if rest.starts_with("</script") {
                in_script = false;
            } else if rest.starts_with("</style") {
                in_style = false;
            }
            i += 1;
            continue;
        }
        if in_tag {
            if chars[i] == '>' {
                in_tag = false;
            }
            i += 1;
            continue;
        }
        if in_script || in_style {
            i += 1;
            continue;
        }
        // Decode common HTML entities
        if chars[i] == '&' {
            let rest: String = chars[i..].iter().take(10).collect();
            if rest.starts_with("&amp;") {
                out.push('&');
                i += 5;
                continue;
            } else if rest.starts_with("&lt;") {
                out.push('<');
                i += 4;
                continue;
            } else if rest.starts_with("&gt;") {
                out.push('>');
                i += 4;
                continue;
            } else if rest.starts_with("&quot;") {
                out.push('"');
                i += 6;
                continue;
            } else if rest.starts_with("&#") {
                // Skip numeric entity
                if let Some(end) = rest.find(';') {
                    i += end + 1;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }

    out
}

/// Controlla se la stringa contiene un numero seguito da un'unità di misura.
fn contains_number_with_unit(s: &str) -> bool {
    let units = [
        "km", "kg", "m/s", "mph", "celsius", "fahrenheit", "metres", "meters",
        "miles", "pounds", "tons", "tonnes", "billion", "million", "trillion",
        "percent", "%", "years", "century", "centuries", "degrees",
        "square", "cubic", "litres", "liters", "watts", "volts",
    ];
    let has_number = s.bytes().any(|b| b.is_ascii_digit());
    has_number && units.iter().any(|u| s.to_lowercase().contains(u))
}

/// Estrae la prima frase da un testo.
fn first_sentence(text: &str) -> String {
    // Trova il primo punto seguito da spazio o fine stringa
    for (i, c) in text.char_indices() {
        if c == '.' && i > 15 {
            let after = text.get(i + 1..i + 2).unwrap_or(" ");
            if after == " " || after == "\n" || text.len() == i + 1 {
                return text[..=i].to_string();
            }
        }
    }
    text.to_string()
}

/// Scarica una pagina Wikipedia e ne estrae i fatti.
pub async fn crawl_wikipedia_page(page: &str, domain: &str) -> Vec<ExtractedFact> {
    let url = format!(
        "https://en.wikipedia.org/wiki/{page}"
    );

    tracing::info!("Crawling Wikipedia: {page}");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("VarcaviaBot/0.1 (https://github.com/VARCAVIA-Git/VARCAVIA; fact-verification)")
        .build()
        .unwrap_or_default();

    match client.get(&url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                tracing::warn!("Wikipedia {page}: HTTP {}", resp.status());
                return fallback_facts(page, domain);
            }
            match resp.text().await {
                Ok(html) => {
                    let facts = extract_facts_from_html(&html, domain, page);
                    if facts.is_empty() {
                        tracing::warn!("Nessun fatto estratto da {page}, uso fallback");
                        return fallback_facts(page, domain);
                    }
                    tracing::info!("Estratti {} fatti da {page}", facts.len());
                    facts
                }
                Err(e) => {
                    tracing::warn!("Errore lettura {page}: {e}");
                    fallback_facts(page, domain)
                }
            }
        }
        Err(e) => {
            tracing::warn!("Errore download {page}: {e}");
            fallback_facts(page, domain)
        }
    }
}

/// Crawla tutte le pagine configurate e restituisce tutti i fatti.
pub async fn crawl_all() -> Vec<ExtractedFact> {
    let mut all_facts = Vec::new();

    for (page, domain) in WIKI_PAGES {
        let facts = crawl_wikipedia_page(page, domain).await;
        all_facts.extend(facts);
    }

    // Aggiungi sempre i fatti hardcoded come baseline
    all_facts.extend(hardcoded_facts());

    // Deduplica
    let mut seen = std::collections::HashSet::new();
    all_facts.retain(|f| seen.insert(f.text.clone()));

    tracing::info!("Totale fatti raccolti: {}", all_facts.len());
    all_facts
}

/// Restituisce tutti i fatti hardcoded come coppie (content, domain).
/// Usato per il seeding diretto senza HTTP.
pub fn get_seed_facts() -> Vec<(String, String)> {
    let mut facts = Vec::new();
    for (page, domain) in WIKI_PAGES {
        for f in fallback_facts(page, domain) {
            facts.push((f.text, f.domain));
        }
    }
    for f in hardcoded_facts() {
        facts.push((f.text, f.domain));
    }
    // Deduplica
    let mut seen = std::collections::HashSet::new();
    facts.retain(|f| seen.insert(f.0.clone()));
    facts
}

/// Fatti di fallback per una specifica pagina Wikipedia.
fn fallback_facts(page: &str, domain: &str) -> Vec<ExtractedFact> {
    let source = format!("wikipedia:{page}");
    let facts: Vec<&str> = match page {
        "Earth" => vec![
            "Earth is the third planet from the Sun and the only astronomical object known to harbor life.",
            "Earth has a mean radius of 6371 kilometres.",
            "Earth orbits the Sun at an average distance of 149.6 million km.",
            "Earth rotates on its axis once every 23 hours 56 minutes and 4 seconds.",
            "The Earth is approximately 4.54 billion years old.",
            "Earth has one natural satellite, the Moon.",
            "About 71 percent of Earth's surface is covered with water.",
            "Earth's atmosphere is composed of 78 percent nitrogen and 21 percent oxygen.",
        ],
        "France" => vec![
            "France is a country in Western Europe with an area of 643,801 square kilometres.",
            "France has a population of approximately 68 million people.",
            "Paris is the capital and largest city of France.",
            "The French Republic was founded in 1792 after the French Revolution.",
            "France is the largest country in the European Union by area.",
            "Mont Blanc at 4,808 metres is the highest peak in France.",
            "France has 13 administrative regions in metropolitan France.",
        ],
        "United_Nations" => vec![
            "The United Nations is an intergovernmental organization founded on 24 October 1945.",
            "The United Nations has 193 member states as of 2024.",
            "The UN headquarters is located in New York City on 18 acres of land.",
            "The United Nations has six principal organs including the General Assembly and Security Council.",
            "The UN Security Council has five permanent members with veto power.",
            "The United Nations was founded to maintain international peace and security.",
        ],
        "Speed_of_light" => vec![
            "The speed of light in vacuum is exactly 299,792,458 metres per second.",
            "Light travels approximately 9.461 trillion kilometres in one year.",
            "The speed of light is the universal speed limit for all massless particles.",
            "Albert Einstein established that nothing can travel faster than the speed of light.",
            "Light takes approximately 8 minutes and 20 seconds to travel from the Sun to Earth.",
            "The speed of light was first measured by Ole Roemer in 1676.",
        ],
        "Water" => vec![
            "Water is a chemical compound with the formula H2O consisting of two hydrogen atoms and one oxygen atom.",
            "Water boils at 100 degrees Celsius at standard atmospheric pressure.",
            "Water freezes at 0 degrees Celsius under standard conditions.",
            "Water covers approximately 71 percent of the Earth's surface.",
            "Pure water has a neutral pH of 7.",
            "Water has a density of approximately 1000 kg per cubic metre at 4 degrees Celsius.",
            "The human body is composed of approximately 60 percent water.",
        ],
        "Albert_Einstein" => vec![
            "Albert Einstein was a German-born theoretical physicist who lived from 1879 to 1955.",
            "Einstein developed the theory of relativity, one of the two pillars of modern physics.",
            "Einstein received the Nobel Prize in Physics in 1921 for his explanation of the photoelectric effect.",
            "Einstein published his special theory of relativity in 1905.",
            "Einstein's mass-energy equivalence formula E=mc2 is the world's most famous equation.",
            "Einstein became a Swiss citizen in 1901 and later an American citizen in 1940.",
        ],
        "Moon" => vec![
            "The Moon is Earth's only natural satellite at a mean distance of 384,400 km.",
            "The Moon has a diameter of 3,474 kilometres, about one-quarter of Earth's diameter.",
            "The Moon orbits Earth once every 27.3 days.",
            "The Moon's surface gravity is about one-sixth of Earth's gravity.",
            "Neil Armstrong became the first person to walk on the Moon on July 20, 1969.",
            "The Moon has no atmosphere and no liquid water on its surface.",
        ],
        "Sun" => vec![
            "The Sun is a G-type main-sequence star comprising 99.86 percent of the Solar System's mass.",
            "The Sun has a surface temperature of approximately 5,778 Kelvin.",
            "The Sun is approximately 4.6 billion years old.",
            "The Sun's diameter is about 1.39 million kilometres, 109 times that of Earth.",
            "The Sun converts approximately 600 million tonnes of hydrogen into helium every second.",
            "Light from the Sun takes about 8 minutes and 20 seconds to reach Earth.",
        ],
        "Human_body" => vec![
            "The adult human body contains approximately 206 bones.",
            "The human heart beats approximately 100,000 times per day.",
            "The human brain contains approximately 86 billion neurons.",
            "The average human body temperature is 37 degrees Celsius.",
            "The human body is composed of approximately 60 percent water.",
            "An adult human body contains about 5 litres of blood.",
        ],
        "DNA" => vec![
            "DNA is a molecule composed of two polynucleotide chains that coil around each other.",
            "The human genome contains approximately 3 billion base pairs of DNA.",
            "DNA was first identified by Friedrich Miescher in 1869.",
            "James Watson and Francis Crick discovered the double helix structure of DNA in 1953.",
            "Human DNA is approximately 99.9 percent identical between individuals.",
            "DNA contains four nucleotide bases: adenine, thymine, guanine, and cytosine.",
        ],
        "Oxygen" => vec![
            "Oxygen is a chemical element with atomic number 8 and symbol O.",
            "Oxygen makes up approximately 21 percent of Earth's atmosphere by volume.",
            "Oxygen is the third most abundant element in the universe by mass.",
            "Oxygen was independently discovered by Carl Wilhelm Scheele and Joseph Priestley in the 1770s.",
            "Liquid oxygen has a boiling point of minus 183 degrees Celsius.",
        ],
        "Gold" => vec![
            "Gold is a chemical element with atomic number 79 and symbol Au.",
            "Gold has a density of 19.3 grams per cubic centimetre.",
            "Gold has a melting point of 1,064 degrees Celsius.",
            "Approximately 190,000 tonnes of gold have been mined throughout history.",
            "Gold is one of the least reactive chemical elements and is resistant to corrosion.",
        ],
        "Python_(programming_language)" => vec![
            "Python is a high-level programming language first released in 1991.",
            "Python was created by Guido van Rossum and named after Monty Python.",
            "Python is one of the most popular programming languages in the world.",
            "Python supports multiple programming paradigms including object-oriented and functional programming.",
            "Python has a comprehensive standard library of over 200 modules.",
        ],
        "Internet" => vec![
            "The Internet is a global system of interconnected computer networks.",
            "The Internet originated from ARPANET, which was established in 1969.",
            "As of 2024, approximately 5.4 billion people worldwide use the Internet.",
            "The World Wide Web was invented by Tim Berners-Lee in 1989.",
            "The Internet uses the TCP/IP protocol suite for data transmission.",
        ],
        "Tokyo" => vec![
            "Tokyo is the capital and largest city of Japan with a population of over 13 million.",
            "The Greater Tokyo Area is the most populous metropolitan area in the world with 37 million people.",
            "Tokyo was originally known as Edo before being renamed in 1868.",
            "Tokyo hosted the Summer Olympic Games in 1964 and 2021.",
            "Tokyo's GDP is approximately 1.9 trillion US dollars, making it the wealthiest city globally.",
        ],
        _ => vec![],
    };

    facts
        .into_iter()
        .map(|text| ExtractedFact {
            text: text.to_string(),
            domain: domain.to_string(),
            source: source.clone(),
        })
        .collect()
}

/// Fatti hardcoded di alta qualita come baseline garantita.
fn hardcoded_facts() -> Vec<ExtractedFact> {
    let facts = vec![
        ("The speed of sound in air at 20 degrees Celsius is approximately 343 metres per second.", "science"),
        ("The Great Wall of China stretches over 21,196 kilometres.", "geography"),
        ("The Amazon River is approximately 6,400 kilometres long.", "geography"),
        ("Mars has two moons named Phobos and Deimos.", "science"),
        ("The Pacific Ocean is the largest and deepest ocean covering 165.25 million square kilometres.", "geography"),
        ("Jupiter is the largest planet in the Solar System with a diameter of 142,984 km.", "science"),
        ("The Mariana Trench is the deepest known part of the ocean at 10,994 metres.", "geography"),
        ("The human genome was first fully sequenced in 2003.", "science"),
        ("The International Space Station orbits Earth at approximately 408 kilometres altitude.", "science"),
        ("Mount Everest is the highest mountain on Earth at 8,849 metres above sea level.", "geography"),
        ("The Sahara Desert covers approximately 9.2 million square kilometres.", "geography"),
        ("The Nile River is approximately 6,650 kilometres long.", "geography"),
        ("The first successful organ transplant was a kidney transplant in 1954.", "health"),
        ("Penicillin was discovered by Alexander Fleming in 1928.", "health"),
        ("The human eye can distinguish approximately 10 million different colours.", "health"),
        ("Carbon dioxide in Earth's atmosphere has exceeded 420 parts per million.", "climate"),
        ("The average global temperature has risen by about 1.1 degrees Celsius since pre-industrial times.", "climate"),
        ("Arctic sea ice has declined by approximately 13 percent per decade since 1979.", "climate"),
        ("Renewable energy sources generated about 30 percent of global electricity in 2023.", "climate"),
        ("The ozone layer is located in the stratosphere at 15 to 35 kilometres altitude.", "climate"),
    ];

    facts
        .into_iter()
        .map(|(text, domain)| ExtractedFact {
            text: text.to_string(),
            domain: domain.to_string(),
            source: "varcavia:hardcoded".to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html() {
        assert_eq!(strip_html("<p>Hello <b>world</b></p>"), "Hello world");
        assert_eq!(strip_html("no tags"), "no tags");
        assert_eq!(strip_html("<script>var x=1;</script>text"), "text");
        assert_eq!(strip_html("a &amp; b"), "a & b");
    }

    #[test]
    fn test_contains_number_with_unit() {
        assert!(contains_number_with_unit("Earth has a radius of 6371 km"));
        assert!(contains_number_with_unit("about 21 percent oxygen"));
        assert!(!contains_number_with_unit("hello world"));
        assert!(!contains_number_with_unit("the number 42"));
    }

    #[test]
    fn test_first_sentence() {
        // Short sentences (under 15 chars to first period) are not split
        assert_eq!(
            first_sentence("Hello world. More text here."),
            "Hello world. More text here."
        );
        assert_eq!(first_sentence("Short"), "Short");
        assert_eq!(
            first_sentence("A sentence with enough characters to pass the minimum. And more."),
            "A sentence with enough characters to pass the minimum."
        );
    }

    #[test]
    fn test_extract_facts_from_html() {
        let html = "<p>Earth is the third planet from the Sun and the only known planet to harbor life. It has a radius of 6371 km and orbits at 149.6 million km from the Sun.</p>";
        let facts = extract_facts_from_html(html, "science", "Earth");
        assert!(!facts.is_empty());
        assert_eq!(facts[0].domain, "science");
    }

    #[test]
    fn test_fallback_facts() {
        let facts = fallback_facts("Earth", "science");
        assert!(facts.len() >= 5);
        assert!(facts[0].text.contains("Earth"));

        let facts = fallback_facts("nonexistent", "test");
        assert!(facts.is_empty());
    }

    #[test]
    fn test_hardcoded_facts() {
        let facts = hardcoded_facts();
        assert!(facts.len() >= 15);
        // Check domains are varied
        let domains: std::collections::HashSet<_> = facts.iter().map(|f| f.domain.as_str()).collect();
        assert!(domains.contains("science"));
        assert!(domains.contains("geography"));
        assert!(domains.contains("health"));
    }

    #[test]
    fn test_fallback_all_pages() {
        // Ensure every configured page has fallback facts
        for (page, domain) in WIKI_PAGES {
            let facts = fallback_facts(page, domain);
            assert!(
                facts.len() >= 4,
                "Page {page} has only {} fallback facts",
                facts.len()
            );
        }
    }
}
