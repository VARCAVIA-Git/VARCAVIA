# VARCAVIA — Guida allo Sviluppo

## Prerequisiti

- **Ubuntu 22.04+** (testato su Acer Aspire 5, 8-16 GB RAM)
- **Rust 1.78+** (installato via rustup)
- **Python 3.11+**
- **just** (task runner, installato via `cargo install just`)

## Setup Iniziale

```bash
# Clona e entra nella directory
git clone <repo-url> varcavia
cd varcavia

# Setup completo (installa tutto, scarica modelli)
bash scripts/setup.sh

# Verifica che tutto compili
just check
```

## Workflow di Sviluppo

### 1. Scegli il task dalla roadmap in CLAUDE.md
Segui l'ordine delle Fasi (1→2→3→4→5→6). Non saltare fasi.

### 2. Crea un branch
```bash
git checkout -b feat/ddna-fingerprint
```

### 3. Implementa con test
Ogni modulo Rust deve avere test inline:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosa_fa() {
        // arrange
        // act
        // assert
    }
}
```

### 4. Verifica
```bash
just lint    # Nessun warning
just test    # Tutti i test passano
just build   # Compila pulito
```

### 5. Commit
```bash
git add -A
git commit -m "feat: implementa ContentFingerprint con BLAKE3+SHA3"
```

## Convenzioni

### File Rust
- Un file per modulo logico (non mega-file da 1000 righe)
- `lib.rs` contiene solo: re-exports, tipi Error, tipo Result
- Ogni funzione pubblica ha doc-comment `///`
- Usa `thiserror` nelle librerie, `anyhow` nei binari

### File Python
- Type hints su tutte le funzioni
- Docstring Google-style
- Max 100 caratteri per riga

### Naming
- Crate Rust: `varcavia-{nome}` (nel Cargo.toml)
- Moduli Rust: `snake_case`
- Tipi Rust: `CamelCase`
- Funzioni/variabili: `snake_case` ovunque

## Come Aggiungere un Nuovo Crate

1. Crea la directory: `mkdir -p crates/nuovo/src`
2. Crea `crates/nuovo/Cargo.toml` con `version.workspace = true`
3. Aggiungi a `Cargo.toml` root: `members = [..., "crates/nuovo"]`
4. Crea `crates/nuovo/src/lib.rs`
5. Verifica: `cargo check -p varcavia-nuovo`

## Come Aggiungere un Nuovo Agent Python

1. Crea `python/agents/nome_agent.py`
2. Estendi `BaseAgent`, implementa `process()`
3. Aggiungi test in `python/tests/test_nome_agent.py`
4. Registra nel nodo Rust (quando il sistema di lancio agenti sarà pronto)

## Debug

### Rust
```bash
# Log dettagliato
RUST_LOG=debug cargo run --bin varcavia-node

# Solo un crate
RUST_LOG=varcavia_ddna=trace cargo run --bin varcavia-node
```

### Python
```bash
# Attiva venv
source .venv/bin/activate

# Test singolo
pytest python/tests/test_agents.py::test_classifier_health -v

# Con log
pytest -s --log-cli-level=DEBUG
```

### Performance
```bash
# Benchmark Rust
cargo bench -p varcavia-ddna

# Profile (richiede flamegraph)
cargo install flamegraph
cargo flamegraph --bin varcavia-node
```

## Risorse

- `CLAUDE.md` — Istruzioni complete per Claude Code (riferimento principale)
- `docs/ARCHITECTURE.md` — Architettura dettagliata
- `docs/PROTOCOLS.md` — Specifiche protocolli
- `docs/API.md` — Documentazione API REST/GraphQL
