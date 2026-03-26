//! # VARCAVIA Crawler
//!
//! Crawler che scarica fatti verificabili da Wikipedia e Wikidata.
//! Usato per popolare il nodo con dati reali.

pub mod wikidata;

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
        // === COUNTRIES (50) ===
        ("The United States of America has a population of approximately 331 million people.", "geography"),
        ("The capital of the United States is Washington, D.C.", "geography"),
        ("The United States has an area of 9,833,520 square kilometres.", "geography"),
        ("China has a population of approximately 1.41 billion people.", "geography"),
        ("The capital of China is Beijing.", "geography"),
        ("China has an area of 9,596,961 square kilometres.", "geography"),
        ("India has a population of approximately 1.43 billion people.", "geography"),
        ("The capital of India is New Delhi.", "geography"),
        ("India has an area of 3,287,263 square kilometres.", "geography"),
        ("Brazil has a population of approximately 214 million people.", "geography"),
        ("The capital of Brazil is Brasilia.", "geography"),
        ("Brazil has an area of 8,515,767 square kilometres.", "geography"),
        ("Russia has an area of 17,098,242 square kilometres, the largest country by area.", "geography"),
        ("The capital of Russia is Moscow.", "geography"),
        ("Indonesia has a population of approximately 275 million people.", "geography"),
        ("The capital of Indonesia is Jakarta.", "geography"),
        ("Nigeria has a population of approximately 218 million people.", "geography"),
        ("The capital of Nigeria is Abuja.", "geography"),
        ("Germany has a population of approximately 84 million people.", "geography"),
        ("The capital of Germany is Berlin.", "geography"),
        ("The United Kingdom has a population of approximately 67 million people.", "geography"),
        ("The capital of the United Kingdom is London.", "geography"),
        ("Canada has an area of 9,984,670 square kilometres.", "geography"),
        ("The capital of Canada is Ottawa.", "geography"),
        ("Australia has an area of 7,692,024 square kilometres.", "geography"),
        ("The capital of Australia is Canberra.", "geography"),
        ("Mexico has a population of approximately 128 million people.", "geography"),
        ("The capital of Mexico is Mexico City.", "geography"),
        ("Italy has a population of approximately 59 million people.", "geography"),
        ("The capital of Italy is Rome.", "geography"),
        ("Spain has a population of approximately 47 million people.", "geography"),
        ("The capital of Spain is Madrid.", "geography"),
        ("South Korea has a population of approximately 51 million people.", "geography"),
        ("The capital of South Korea is Seoul.", "geography"),
        ("Argentina has an area of 2,780,400 square kilometres.", "geography"),
        ("The capital of Argentina is Buenos Aires.", "geography"),
        ("Egypt has a population of approximately 104 million people.", "geography"),
        ("The capital of Egypt is Cairo.", "geography"),
        ("South Africa has a population of approximately 60 million people.", "geography"),
        ("The capital of South Africa is Pretoria.", "geography"),
        ("Saudi Arabia has an area of 2,149,690 square kilometres.", "geography"),
        ("The capital of Saudi Arabia is Riyadh.", "geography"),
        ("Turkey has a population of approximately 85 million people.", "geography"),
        ("The capital of Turkey is Ankara.", "geography"),
        ("Thailand has a population of approximately 72 million people.", "geography"),
        ("The capital of Thailand is Bangkok.", "geography"),
        ("Poland has a population of approximately 38 million people.", "geography"),
        ("The capital of Poland is Warsaw.", "geography"),
        ("Switzerland has a population of approximately 8.7 million people.", "geography"),
        ("The capital of Switzerland is Bern.", "geography"),
        // === SCIENCE (50) ===
        ("The speed of sound in air at 20 degrees Celsius is approximately 343 metres per second.", "science"),
        ("The gravitational constant G is approximately 6.674 times 10 to the minus 11 N m2 kg-2.", "science"),
        ("Avogadro's number is approximately 6.022 times 10 to the 23.", "science"),
        ("The Planck constant is approximately 6.626 times 10 to the minus 34 joule seconds.", "science"),
        ("The charge of an electron is approximately 1.602 times 10 to the minus 19 coulombs.", "science"),
        ("The Boltzmann constant is approximately 1.381 times 10 to the minus 23 joules per kelvin.", "science"),
        ("Absolute zero is minus 273.15 degrees Celsius or 0 Kelvin.", "science"),
        ("The human genome was first fully sequenced in 2003.", "science"),
        ("The International Space Station orbits Earth at approximately 408 kilometres altitude.", "science"),
        ("Mars has two moons named Phobos and Deimos.", "science"),
        ("Jupiter is the largest planet in the Solar System with a diameter of 142,984 km.", "science"),
        ("Newton's first law states that an object at rest stays at rest unless acted upon by an external force.", "science"),
        ("The periodic table was first published by Dmitri Mendeleev in 1869.", "science"),
        ("Photosynthesis converts carbon dioxide and water into glucose and oxygen using sunlight.", "science"),
        ("The double helix structure of DNA was discovered by Watson and Crick in 1953.", "science"),
        ("The theory of general relativity was published by Albert Einstein in 1915.", "science"),
        ("The Higgs boson was discovered at CERN in 2012.", "science"),
        ("The human body contains approximately 37.2 trillion cells.", "science"),
        ("Sound cannot travel through a vacuum.", "science"),
        ("Diamond is the hardest known natural material on the Mohs scale.", "science"),
        ("The chemical formula for table salt is NaCl, sodium chloride.", "science"),
        ("Helium is the second most abundant element in the observable universe.", "science"),
        ("The pH scale ranges from 0 to 14, with 7 being neutral.", "science"),
        ("The speed of light in a vacuum is exactly 299,792,458 metres per second.", "science"),
        ("E equals mc squared is the mass-energy equivalence formula.", "science"),
        ("A light-year is the distance light travels in one year, about 9.461 trillion kilometres.", "science"),
        ("The Milky Way galaxy contains an estimated 100 to 400 billion stars.", "science"),
        ("Neutron stars can spin at up to 716 rotations per second.", "science"),
        ("The atomic number of carbon is 6.", "science"),
        ("The chemical formula for water is H2O.", "science"),
        ("Pi is approximately 3.14159265358979.", "science"),
        ("The square root of 2 is approximately 1.41421356.", "science"),
        ("One astronomical unit is approximately 149.6 million kilometres.", "science"),
        ("The mass of the proton is approximately 1.673 times 10 to the minus 27 kilograms.", "science"),
        ("Gravity accelerates objects at approximately 9.81 metres per second squared on Earth.", "science"),
        ("The cosmic microwave background radiation was discovered in 1965 by Penzias and Wilson.", "science"),
        ("Antibiotics do not work against viruses.", "science"),
        ("The largest known prime number as of 2024 has more than 41 million digits.", "science"),
        ("RNA stands for ribonucleic acid.", "science"),
        ("Mitochondria are known as the powerhouses of the cell.", "science"),
        ("The human body has 206 bones in adulthood.", "science"),
        ("The boiling point of ethanol is 78.37 degrees Celsius.", "science"),
        ("Iron has the chemical symbol Fe and atomic number 26.", "science"),
        ("The freezing point of mercury is minus 38.83 degrees Celsius.", "science"),
        ("Copper has the chemical symbol Cu and atomic number 29.", "science"),
        ("The half-life of carbon-14 is approximately 5,730 years.", "science"),
        ("The observable universe has a diameter of approximately 93 billion light-years.", "science"),
        ("Saturn has a density lower than water at 0.687 grams per cubic centimetre.", "science"),
        ("Venus rotates in the opposite direction to most planets in the Solar System.", "science"),
        ("Titan is the largest moon of Saturn.", "science"),
        // === GEOGRAPHY (50) ===
        ("The Pacific Ocean is the largest and deepest ocean covering 165.25 million square kilometres.", "geography"),
        ("The Atlantic Ocean has an area of approximately 106.46 million square kilometres.", "geography"),
        ("The Indian Ocean has an area of approximately 70.56 million square kilometres.", "geography"),
        ("The Arctic Ocean is the smallest ocean with an area of 14.06 million square kilometres.", "geography"),
        ("The Great Wall of China stretches over 21,196 kilometres.", "geography"),
        ("The Amazon River is approximately 6,400 kilometres long.", "geography"),
        ("The Nile River is approximately 6,650 kilometres long.", "geography"),
        ("The Sahara Desert covers approximately 9.2 million square kilometres.", "geography"),
        ("Mount Everest is the highest mountain on Earth at 8,849 metres above sea level.", "geography"),
        ("The Mariana Trench is the deepest known part of the ocean at 10,994 metres.", "geography"),
        ("Lake Baikal in Russia is the deepest lake in the world at 1,642 metres.", "geography"),
        ("The Dead Sea is the lowest point on Earth's surface at 430 metres below sea level.", "geography"),
        ("The Andes is the longest continental mountain range at approximately 7,000 kilometres.", "geography"),
        ("The Yangtze River is the longest river in Asia at 6,300 kilometres.", "geography"),
        ("The Mississippi River is approximately 3,730 kilometres long.", "geography"),
        ("The Gobi Desert covers approximately 1.3 million square kilometres.", "geography"),
        ("The Caspian Sea is the largest enclosed inland body of water with 371,000 square kilometres.", "geography"),
        ("Lake Superior is the largest of the Great Lakes with an area of 82,100 square kilometres.", "geography"),
        ("The Congo River is the deepest river in the world at over 220 metres.", "geography"),
        ("The Danube River flows through 10 countries in Europe.", "geography"),
        ("K2 is the second highest mountain in the world at 8,611 metres.", "geography"),
        ("Mount Kilimanjaro is the highest mountain in Africa at 5,895 metres.", "geography"),
        ("The Panama Canal is approximately 82 kilometres long.", "geography"),
        ("The Suez Canal is approximately 193 kilometres long.", "geography"),
        ("Antarctica is the coldest continent with temperatures reaching minus 89.2 degrees Celsius.", "geography"),
        ("The Amazon Rainforest covers approximately 5.5 million square kilometres.", "geography"),
        ("The Great Barrier Reef is approximately 2,300 kilometres long.", "geography"),
        ("The Atacama Desert in Chile is one of the driest places on Earth.", "geography"),
        ("The Rhine River is approximately 1,230 kilometres long.", "geography"),
        ("Victoria Falls has a width of 1,708 metres and height of 108 metres.", "geography"),
        ("Greenland is the largest island in the world with an area of 2,166,086 square kilometres.", "geography"),
        ("The Mediterranean Sea has an area of approximately 2.5 million square kilometres.", "geography"),
        ("The Ganges River is approximately 2,525 kilometres long.", "geography"),
        ("Mount Fuji is the highest mountain in Japan at 3,776 metres.", "geography"),
        ("The Sahel region stretches across 5,400 kilometres of Africa.", "geography"),
        ("Lake Victoria is the largest lake in Africa with 68,800 square kilometres.", "geography"),
        ("The Mekong River is approximately 4,350 kilometres long.", "geography"),
        ("The Alps stretch approximately 1,200 kilometres across Europe.", "geography"),
        ("Angel Falls in Venezuela is the tallest waterfall at 979 metres.", "geography"),
        ("The Black Sea has an area of approximately 436,400 square kilometres.", "geography"),
        ("Madagascar is the fourth largest island in the world.", "geography"),
        ("The Himalayas contain 14 peaks above 8,000 metres.", "geography"),
        ("The Volga River is the longest river in Europe at 3,530 kilometres.", "geography"),
        ("The Caribbean Sea has an area of approximately 2.75 million square kilometres.", "geography"),
        ("The Strait of Gibraltar is approximately 14.3 kilometres wide at its narrowest point.", "geography"),
        ("Lake Titicaca is the highest navigable lake in the world at 3,812 metres elevation.", "geography"),
        ("The Zambezi River is approximately 2,574 kilometres long.", "geography"),
        ("The Ross Ice Shelf in Antarctica has an area of approximately 487,000 square kilometres.", "geography"),
        ("The Pyrenees mountain range is approximately 491 kilometres long.", "geography"),
        ("The Aral Sea has lost approximately 90 percent of its volume since the 1960s.", "geography"),
        // === HISTORY (50) ===
        ("The French Revolution began in 1789 with the storming of the Bastille.", "history"),
        ("World War I lasted from 1914 to 1918.", "history"),
        ("World War II lasted from 1939 to 1945.", "history"),
        ("The Berlin Wall fell on November 9, 1989.", "history"),
        ("The Declaration of Independence was signed on July 4, 1776.", "history"),
        ("The Roman Empire fell in 476 AD.", "history"),
        ("The printing press was invented by Johannes Gutenberg around 1440.", "history"),
        ("Christopher Columbus reached the Americas in 1492.", "history"),
        ("The Magna Carta was signed in 1215.", "history"),
        ("Neil Armstrong walked on the Moon on July 20, 1969.", "history"),
        ("The Soviet Union dissolved on December 26, 1991.", "history"),
        ("The Renaissance began in Italy in the 14th century.", "history"),
        ("The Industrial Revolution began in Britain in the late 18th century.", "history"),
        ("The United Nations was founded on October 24, 1945.", "history"),
        ("The Treaty of Versailles was signed on June 28, 1919.", "history"),
        ("The Panama Canal opened on August 15, 1914.", "history"),
        ("The Suez Canal opened on November 17, 1869.", "history"),
        ("The Titanic sank on April 15, 1912.", "history"),
        ("The Great Fire of London occurred in 1666.", "history"),
        ("The Rosetta Stone was discovered in 1799.", "history"),
        ("Martin Luther King Jr. delivered his I Have a Dream speech on August 28, 1963.", "history"),
        ("The abolition of slavery in the United States was ratified in 1865 with the 13th Amendment.", "history"),
        ("The Black Death killed approximately one third of Europe's population in the 14th century.", "history"),
        ("The first Olympic Games of the modern era were held in Athens in 1896.", "history"),
        ("The Chernobyl nuclear disaster occurred on April 26, 1986.", "history"),
        ("The European Union was established by the Maastricht Treaty in 1993.", "history"),
        ("The Marshall Plan was enacted in 1948 to aid post-war European recovery.", "history"),
        ("India gained independence from Britain on August 15, 1947.", "history"),
        ("The Chinese Communist Revolution was established on October 1, 1949.", "history"),
        ("The Cuban Missile Crisis occurred in October 1962.", "history"),
        ("The Vietnam War ended with the fall of Saigon on April 30, 1975.", "history"),
        ("Nelson Mandela was released from prison on February 11, 1990.", "history"),
        ("The first photograph was taken by Joseph Niepce in 1826.", "history"),
        ("The Gutenberg Bible was the first major book printed in Europe around 1455.", "history"),
        ("The Wright brothers made the first powered flight on December 17, 1903.", "history"),
        ("The Emancipation Proclamation was issued by Abraham Lincoln on January 1, 1863.", "history"),
        ("The Sistine Chapel ceiling was painted by Michelangelo between 1508 and 1512.", "history"),
        ("The first transatlantic telegraph cable was completed in 1866.", "history"),
        ("Women gained the right to vote in the United States in 1920 with the 19th Amendment.", "history"),
        ("The atomic bomb was first used in warfare on Hiroshima on August 6, 1945.", "history"),
        ("The Hubble Space Telescope was launched on April 24, 1990.", "history"),
        ("The Human Genome Project was completed in 2003.", "history"),
        ("The first successful heart transplant was performed by Christiaan Barnard in 1967.", "history"),
        ("Alexander the Great died in 323 BC at the age of 32.", "history"),
        ("The construction of the Great Pyramid of Giza was completed around 2560 BC.", "history"),
        ("The Spanish Armada was defeated by England in 1588.", "history"),
        ("Napoleon Bonaparte was exiled to Saint Helena in 1815.", "history"),
        ("The first Geneva Convention was adopted in 1864.", "history"),
        ("The League of Nations was established in 1920.", "history"),
        ("The Korean War lasted from 1950 to 1953.", "history"),
        // === TECHNOLOGY (50) ===
        ("The World Wide Web was invented by Tim Berners-Lee in 1989.", "technology"),
        ("The first email was sent by Ray Tomlinson in 1971.", "technology"),
        ("The iPhone was first released by Apple on June 29, 2007.", "technology"),
        ("Google was founded by Larry Page and Sergey Brin on September 4, 1998.", "technology"),
        ("The first website went live on August 6, 1991.", "technology"),
        ("The transistor was invented at Bell Labs in 1947.", "technology"),
        ("The first programmable computer, the Z3, was built by Konrad Zuse in 1941.", "technology"),
        ("ARPANET, the precursor to the Internet, was established in 1969.", "technology"),
        ("The Linux kernel was first released by Linus Torvalds on September 17, 1991.", "technology"),
        ("Bitcoin was created by Satoshi Nakamoto in 2009.", "technology"),
        ("The first smartphone was the IBM Simon, released in 1994.", "technology"),
        ("Amazon was founded by Jeff Bezos on July 5, 1994.", "technology"),
        ("Facebook was launched by Mark Zuckerberg on February 4, 2004.", "technology"),
        ("Wikipedia was launched on January 15, 2001.", "technology"),
        ("The GPS system became fully operational on April 27, 1995.", "technology"),
        ("The USB standard was introduced in 1996.", "technology"),
        ("Netflix was founded on August 29, 1997.", "technology"),
        ("Spotify was launched on October 7, 2008.", "technology"),
        ("The first 3D printer was created by Chuck Hull in 1984.", "technology"),
        ("Wi-Fi was introduced to consumers in 1999.", "technology"),
        ("Python programming language was first released in 1991.", "technology"),
        ("Java programming language was released by Sun Microsystems in 1995.", "technology"),
        ("The Rust programming language was first released in 2010.", "technology"),
        ("Tesla was founded on July 1, 2003.", "technology"),
        ("SpaceX was founded by Elon Musk on March 14, 2002.", "technology"),
        ("The first artificial satellite, Sputnik 1, was launched on October 4, 1957.", "technology"),
        ("Bluetooth technology was invented in 1994 by Ericsson.", "technology"),
        ("YouTube was launched on February 14, 2005.", "technology"),
        ("Twitter was launched on March 21, 2006.", "technology"),
        ("The Kindle e-reader was released by Amazon on November 19, 2007.", "technology"),
        ("Instagram was launched on October 6, 2010.", "technology"),
        ("The first commercial jet airliner was the de Havilland Comet in 1952.", "technology"),
        ("The compact disc was commercially released in 1982.", "technology"),
        ("The World Wide Web Consortium (W3C) was founded on October 1, 1994.", "technology"),
        ("OpenAI was founded on December 11, 2015.", "technology"),
        ("ChatGPT was launched on November 30, 2022.", "technology"),
        ("The first computer mouse was invented by Douglas Engelbart in 1964.", "technology"),
        ("HTTPS protocol was created by Netscape in 1994.", "technology"),
        ("The PDF format was created by Adobe in 1993.", "technology"),
        ("The MP3 audio format was standardized in 1993.", "technology"),
        ("The first video call was made by AT&T in 1970.", "technology"),
        ("Ethernet was invented by Robert Metcalfe at Xerox PARC in 1973.", "technology"),
        ("The Domain Name System was introduced in 1985.", "technology"),
        ("The first solar cell was created at Bell Labs in 1954.", "technology"),
        ("CRISPR gene editing technology was first used in 2012.", "technology"),
        ("The Large Hadron Collider at CERN began operations in 2008.", "technology"),
        ("Quantum supremacy was first demonstrated by Google in 2019.", "technology"),
        ("The James Webb Space Telescope was launched on December 25, 2021.", "technology"),
        ("The first nuclear power plant began operation in Obninsk, Russia in 1954.", "technology"),
        ("The barcode was first commercially used on June 26, 1974.", "technology"),
        // === HEALTH (50) ===
        ("The average human heart beats approximately 100,000 times per day.", "health"),
        ("The average human body temperature is 37 degrees Celsius.", "health"),
        ("The human body is composed of approximately 60 percent water.", "health"),
        ("An adult human body contains about 5 litres of blood.", "health"),
        ("The human brain contains approximately 86 billion neurons.", "health"),
        ("Penicillin was discovered by Alexander Fleming in 1928.", "health"),
        ("The first successful organ transplant was a kidney transplant in 1954.", "health"),
        ("The human eye can distinguish approximately 10 million different colours.", "health"),
        ("Global life expectancy at birth is approximately 73 years as of 2023.", "health"),
        ("Cardiovascular disease is the leading cause of death worldwide.", "health"),
        ("Approximately 422 million people worldwide have diabetes.", "health"),
        ("The WHO estimates that 280 million people globally suffer from depression.", "health"),
        ("Malaria kills approximately 620,000 people per year.", "health"),
        ("The average adult human has about 206 bones.", "health"),
        ("The human liver can regenerate to its full size from as little as 25 percent.", "health"),
        ("The small intestine is approximately 6 metres long in an adult.", "health"),
        ("Red blood cells live for approximately 120 days.", "health"),
        ("The human body produces about 1 litre of saliva per day.", "health"),
        ("The human nose can detect approximately 1 trillion different scents.", "health"),
        ("The fastest muscle in the human body is the orbicularis oculi, which blinks the eye.", "health"),
        ("The femur is the longest and strongest bone in the human body.", "health"),
        ("Human teeth are as hard as shark teeth.", "health"),
        ("The average adult breathes approximately 20,000 times per day.", "health"),
        ("The human skin is the largest organ of the body.", "health"),
        ("Approximately 8 million people die from cancer each year worldwide.", "health"),
        ("Tuberculosis kills approximately 1.3 million people per year.", "health"),
        ("HIV has infected approximately 85 million people since the start of the pandemic.", "health"),
        ("The first vaccine was developed by Edward Jenner in 1796 for smallpox.", "health"),
        ("Smallpox was declared eradicated by the WHO in 1980.", "health"),
        ("The human genome contains approximately 20,000 to 25,000 genes.", "health"),
        ("An adult human produces approximately 200 billion red blood cells per day.", "health"),
        ("The human body contains approximately 640 muscles.", "health"),
        ("Vitamin C deficiency causes scurvy.", "health"),
        ("The normal resting heart rate for adults is 60 to 100 beats per minute.", "health"),
        ("Type 2 diabetes accounts for approximately 90 percent of all diabetes cases.", "health"),
        ("The average human brain weighs approximately 1.4 kilograms.", "health"),
        ("Antibiotics were first widely used in the 1940s.", "health"),
        ("The placebo effect can cause measurable physiological changes.", "health"),
        ("Approximately 1 billion people worldwide are affected by hypertension.", "health"),
        ("The WHO recommends at least 150 minutes of moderate exercise per week.", "health"),
        ("Smoking causes approximately 8 million deaths per year worldwide.", "health"),
        ("Approximately 2.4 billion people lack access to basic sanitation facilities.", "health"),
        ("The first successful blood transfusion was performed in 1818.", "health"),
        ("The polio vaccine was developed by Jonas Salk in 1955.", "health"),
        ("Aspirin was first synthesized by Felix Hoffmann in 1897.", "health"),
        ("The average human fingernail grows approximately 3.5 millimetres per month.", "health"),
        ("The cornea is the only part of the human body without a blood supply.", "health"),
        ("Human stomach acid has a pH between 1.5 and 3.5.", "health"),
        ("The thyroid gland regulates metabolism, growth, and development.", "health"),
        ("Approximately 300 million people worldwide suffer from asthma.", "health"),
        // === ASTRONOMY (50) ===
        ("The Sun is approximately 4.6 billion years old.", "science"),
        ("The nearest star to Earth after the Sun is Proxima Centauri at 4.24 light-years.", "science"),
        ("The Andromeda Galaxy is approximately 2.537 million light-years from Earth.", "science"),
        ("A black hole is a region of spacetime where gravity is so strong that nothing can escape.", "science"),
        ("The Hubble Space Telescope orbits Earth at approximately 547 kilometres altitude.", "science"),
        ("The Sun's core temperature is approximately 15 million degrees Celsius.", "science"),
        ("Mercury is the smallest planet in the Solar System with a diameter of 4,879 km.", "science"),
        ("Neptune has the strongest winds in the Solar System at up to 2,100 km/h.", "science"),
        ("Pluto was reclassified as a dwarf planet in 2006.", "science"),
        ("The Voyager 1 spacecraft is the most distant human-made object from Earth.", "science"),
        ("A pulsar is a highly magnetized rotating neutron star.", "science"),
        ("Ganymede is the largest moon in the Solar System.", "science"),
        ("The asteroid belt is located between the orbits of Mars and Jupiter.", "science"),
        ("The Oort Cloud is estimated to extend up to 100,000 AU from the Sun.", "science"),
        ("Halley's Comet is visible from Earth approximately every 75 to 79 years.", "science"),
        ("The Kuiper Belt extends from 30 to 50 AU from the Sun.", "science"),
        ("Betelgeuse is a red supergiant star approximately 700 light-years from Earth.", "science"),
        ("The Great Red Spot on Jupiter is a storm that has lasted for over 350 years.", "science"),
        ("Europa, a moon of Jupiter, may have a subsurface ocean.", "science"),
        ("The solar wind travels at approximately 400 kilometres per second.", "science"),
        ("A supernova can briefly outshine an entire galaxy.", "science"),
        ("The Crab Nebula is the remnant of a supernova observed in 1054 AD.", "science"),
        ("The Milky Way is a barred spiral galaxy.", "science"),
        ("The cosmic microwave background has a temperature of approximately 2.725 Kelvin.", "science"),
        ("Dark matter makes up approximately 27 percent of the universe.", "science"),
        ("Dark energy makes up approximately 68 percent of the universe.", "science"),
        ("The age of the universe is approximately 13.8 billion years.", "science"),
        ("Olympus Mons on Mars is the tallest volcano in the Solar System at 21.9 km.", "science"),
        ("The rings of Saturn are primarily composed of ice particles and rocky debris.", "science"),
        ("The Chandrasekhar limit is approximately 1.4 solar masses.", "science"),
        ("White dwarf stars are the remnants of low to medium mass stars.", "science"),
        ("Io is the most volcanically active body in the Solar System.", "science"),
        ("The heliosphere extends approximately 120 AU from the Sun.", "science"),
        ("Sirius is the brightest star in the night sky.", "science"),
        ("The Carina Nebula is one of the largest nebulae in the Milky Way.", "science"),
        ("Mercury's surface temperature ranges from minus 180 to 430 degrees Celsius.", "science"),
        ("The average distance from Earth to the Moon is 384,400 kilometres.", "science"),
        ("The Sun loses approximately 4 million tonnes of mass per second through fusion.", "science"),
        ("Uranus has 27 known moons.", "science"),
        ("The Trappist-1 system has 7 Earth-sized planets.", "science"),
        ("A magnetar has a magnetic field approximately 1 quadrillion times stronger than Earth's.", "science"),
        ("The Horsehead Nebula is located in the constellation Orion.", "science"),
        ("Kepler-452b is sometimes called Earth's cousin due to its similar size and orbit.", "science"),
        ("The Parker Solar Probe is the fastest human-made object ever built.", "science"),
        ("Titan has a thick atmosphere composed primarily of nitrogen.", "science"),
        ("The Virgo Supercluster contains approximately 100 galaxy groups.", "science"),
        ("Neutron stars have a density of approximately 4 times 10 to the 17 kg per cubic metre.", "science"),
        ("The first exoplanet was discovered in 1992 orbiting a pulsar.", "science"),
        ("There are approximately 2 trillion galaxies in the observable universe.", "science"),
        ("The Bootes Void is one of the largest known voids in the universe at 330 million light-years.", "science"),
        // === CLIMATE (20) ===
        ("Carbon dioxide in Earth's atmosphere has exceeded 420 parts per million.", "climate"),
        ("The average global temperature has risen by about 1.1 degrees Celsius since pre-industrial times.", "climate"),
        ("Arctic sea ice has declined by approximately 13 percent per decade since 1979.", "climate"),
        ("Renewable energy sources generated about 30 percent of global electricity in 2023.", "climate"),
        ("The ozone layer is located in the stratosphere at 15 to 35 kilometres altitude.", "climate"),
        ("Sea level has risen approximately 21 to 24 centimetres since 1880.", "climate"),
        ("The Paris Agreement was adopted by 196 parties in December 2015.", "climate"),
        ("Methane is approximately 80 times more potent than CO2 as a greenhouse gas over 20 years.", "climate"),
        ("Deforestation accounts for approximately 10 percent of global greenhouse gas emissions.", "climate"),
        ("Antarctica contains approximately 26.5 million cubic kilometres of ice.", "climate"),
        ("The Amazon Rainforest absorbs approximately 2 billion tonnes of CO2 per year.", "climate"),
        ("Global coal consumption peaked at approximately 8.3 billion tonnes in 2023.", "climate"),
        ("The Greenland ice sheet has lost approximately 270 billion tonnes of ice per year since 2002.", "climate"),
        ("Ocean acidification has increased by approximately 26 percent since the Industrial Revolution.", "climate"),
        ("Coral reefs support approximately 25 percent of all marine species.", "climate"),
        ("Solar energy capacity has grown from 40 GW in 2010 to over 1,000 GW in 2023.", "climate"),
        ("Wind energy provided approximately 7 percent of global electricity in 2023.", "climate"),
        ("The Earth's average surface temperature is approximately 15 degrees Celsius.", "climate"),
        ("Approximately 40 percent of the world's population lives within 100 km of a coast.", "climate"),
        ("Permafrost covers approximately 25 percent of the Northern Hemisphere's land surface.", "climate"),
        ("Global mean sea level is rising at approximately 3.6 mm per year.", "climate"),
        ("The 10 warmest years on record have all occurred since 2010.", "climate"),
        ("Approximately 1 million species are at risk of extinction due to climate change.", "climate"),
        ("Electric vehicles accounted for approximately 18 percent of global car sales in 2023.", "climate"),
        ("The Earth absorbs approximately 240 watts per square metre of solar radiation.", "climate"),
        // === ADDITIONAL FACTS (30) ===
        ("The speed of the Earth's rotation at the equator is approximately 1,670 km/h.", "science"),
        ("The deepest point drilled into the Earth's crust is the Kola Superdeep Borehole at 12,262 metres.", "science"),
        ("The average distance from the Sun to Saturn is approximately 1.43 billion kilometres.", "science"),
        ("Light from the Sun takes approximately 8 minutes and 20 seconds to reach Earth.", "science"),
        ("The Richter scale measures earthquake magnitude on a logarithmic scale.", "science"),
        ("The tallest building in the world is the Burj Khalifa at 828 metres.", "geography"),
        ("The English Channel is approximately 33.3 kilometres wide at its narrowest point.", "geography"),
        ("Singapore has an area of approximately 733 square kilometres.", "geography"),
        ("Monaco is the most densely populated country in the world.", "geography"),
        ("Vatican City is the smallest country in the world at 0.44 square kilometres.", "geography"),
        ("The Trans-Siberian Railway is approximately 9,289 kilometres long.", "geography"),
        ("The Eurostar tunnel under the English Channel is 50.45 kilometres long.", "geography"),
        ("The Apollo 11 mission landed on the Moon on July 20, 1969.", "history"),
        ("The Voyager 1 spacecraft was launched on September 5, 1977.", "history"),
        ("The first programmable electronic computer, Colossus, was built in 1943.", "history"),
        ("The discovery of X-rays by Wilhelm Roentgen occurred in 1895.", "history"),
        ("The first telephone call was made by Alexander Graham Bell on March 10, 1876.", "history"),
        ("The Eiffel Tower was completed on March 31, 1889.", "history"),
        ("The Concorde made its first commercial flight on January 21, 1976.", "history"),
        ("The International Space Station has been continuously occupied since November 2, 2000.", "technology"),
        ("The human genome contains approximately 3.2 billion base pairs.", "science"),
        ("Approximately 71 percent of the Earth's surface is covered by water.", "science"),
        ("The deepest part of the ocean is the Challenger Deep at 10,994 metres.", "science"),
        ("The atmospheric pressure at sea level is approximately 101,325 pascals.", "science"),
        ("The global population reached 8 billion on November 15, 2022.", "geography"),
    ];

    facts
        .into_iter()
        .map(|(text, domain)| ExtractedFact {
            text: text.to_string(),
            domain: domain.to_string(),
            source: "varcavia:curated".to_string(),
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
        assert!(facts.len() >= 400, "Expected 400+ hardcoded facts, got {}", facts.len());
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
