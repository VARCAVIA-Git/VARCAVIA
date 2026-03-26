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

    Ok(())
}

pub async fn handle_status() -> anyhow::Result<()> {
    tracing::info!("Stato nodo: TODO — implementare nella Fase 1");
    Ok(())
}
