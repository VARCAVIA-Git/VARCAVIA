//! Comandi CLI del nodo VARCAVIA.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Inizializza un nuovo nodo
    Init {
        /// Directory per i dati del nodo
        #[arg(short, long, default_value = "~/varcavia-data")]
        data_dir: String,
    },
    /// Mostra lo stato del nodo
    Status,
    /// Popola il nodo con fatti reali da Wikipedia
    Seed {
        /// Porta del nodo API a cui inviare i fatti
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

pub async fn handle_init(data_dir: &str) -> anyhow::Result<()> {
    let expanded = shellexpand::tilde(data_dir);
    let path = std::path::Path::new(expanded.as_ref());
    std::fs::create_dir_all(path)?;
    tracing::info!("Nodo inizializzato in: {}", path.display());

    // Genera keypair per il nodo
    let keypair = varcavia_ddna::identity::KeyPair::generate();
    let pubkey_hex = hex::encode(keypair.public_key_bytes());
    tracing::info!("Node ID (pubkey): {}", pubkey_hex);

    // Salva chiave privata
    let key_path = path.join("node_key.secret");
    std::fs::write(&key_path, keypair.secret_bytes())?;
    tracing::info!("Chiave privata salvata in: {}", key_path.display());

    // Crea directory per il database
    let db_path = path.join("db");
    std::fs::create_dir_all(&db_path)?;
    tracing::info!("Database directory: {}", db_path.display());

    println!("Nodo inizializzato con successo!");
    println!("  Node ID: {pubkey_hex}");
    println!("  Data dir: {}", path.display());
    println!("  Avvia con: cargo run --bin varcavia-node");

    Ok(())
}

pub async fn handle_status() -> anyhow::Result<()> {
    let data_dir = shellexpand::tilde("~/varcavia-data").to_string();
    let key_path = format!("{data_dir}/node_key.secret");

    if !std::path::Path::new(&key_path).exists() {
        println!("Nodo non inizializzato. Esegui: varcavia-node init");
        return Ok(());
    }

    let secret_bytes = std::fs::read(&key_path)?;
    let secret: [u8; 32] = secret_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Chiave corrotta"))?;
    let keypair = varcavia_ddna::identity::KeyPair::from_bytes(&secret);
    let node_id = hex::encode(keypair.public_key_bytes());

    let db_path = format!("{data_dir}/db");
    let data_count = if std::path::Path::new(&db_path).exists() {
        let db = sled::open(&db_path)?;
        db.scan_prefix(b"d:").count()
    } else {
        0
    };

    println!("VARCAVIA Node Status");
    println!("  Node ID:    {node_id}");
    println!("  Data dir:   {data_dir}");
    println!("  Data count: {data_count}");
    println!("  Status:     offline (avvia con: cargo run --bin varcavia-node)");

    Ok(())
}

/// Popola il nodo con fatti reali da Wikipedia.
pub async fn handle_seed(port: u16) -> anyhow::Result<()> {
    println!("VARCAVIA Seed — Popolo il nodo con fatti reali...");
    println!("  Target: http://127.0.0.1:{port}/api/v1/data");
    println!();

    // Verifica che il nodo sia raggiungibile
    let base_url = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    match client.get(format!("{base_url}/api/v1/node/status")).send().await {
        Ok(r) if r.status().is_success() => {
            println!("  Nodo raggiungibile.");
        }
        _ => {
            println!("  ERRORE: nodo non raggiungibile su porta {port}.");
            println!("  Avvia il nodo prima con: cargo run --bin varcavia-node -- --port {port}");
            return Ok(());
        }
    }

    // Crawla e inserisci
    let facts = varcavia_crawler::crawl_all().await;
    let total = facts.len();
    let mut inserted = 0u64;
    let mut duplicates = 0u64;
    let mut errors = 0u64;

    for (i, fact) in facts.iter().enumerate() {
        let body = serde_json::json!({
            "content": fact.text,
            "domain": fact.domain,
            "source": fact.source,
        });

        match client
            .post(format!("{base_url}/api/v1/data"))
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    inserted += 1;
                    if (i + 1) % 10 == 0 || i + 1 == total {
                        println!("  [{}/{}] Inseriti: {inserted} | Duplicati: {duplicates}", i + 1, total);
                    }
                } else if status.as_u16() == 409 {
                    duplicates += 1;
                } else {
                    errors += 1;
                    tracing::warn!("Errore inserimento fatto {}: HTTP {}", i, status);
                }
            }
            Err(e) => {
                errors += 1;
                tracing::warn!("Errore connessione: {e}");
            }
        }
    }

    println!();
    println!("Fase 1 completata (Wikipedia):");
    println!("  Totale: {total} | Inseriti: {inserted} | Duplicati: {duplicates} | Errori: {errors}");

    // Fase 2: Wikidata SPARQL (best-effort)
    println!();
    println!("Fase 2: Wikidata SPARQL...");
    let wd_facts = varcavia_crawler::wikidata::crawl_wikidata().await;
    let wd_total = wd_facts.len();
    let mut wd_inserted = 0u64;
    let mut wd_duplicates = 0u64;

    for (i, fact) in wd_facts.iter().enumerate() {
        let body = serde_json::json!({
            "content": fact.text,
            "domain": fact.domain,
            "source": fact.source,
        });

        match client.post(format!("{base_url}/api/v1/data")).json(&body).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    wd_inserted += 1;
                } else if resp.status().as_u16() == 409 {
                    wd_duplicates += 1;
                }
            }
            Err(_) => {}
        }

        if (i + 1) % 100 == 0 || i + 1 == wd_total {
            println!("  [{}/{}] Inseriti: {wd_inserted}", i + 1, wd_total);
        }
    }

    println!();
    println!("Seed completato!");
    println!("  Wikipedia:  {inserted} inseriti / {total} totali");
    println!("  Wikidata:   {wd_inserted} inseriti / {wd_total} totali");
    println!("  Totale DB:  ~ {} fatti", inserted + wd_inserted);

    Ok(())
}
