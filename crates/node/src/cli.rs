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
