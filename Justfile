# ════════════════════════════════════════════════════════════
# VARCAVIA — Justfile (Task Runner)
# ════════════════════════════════════════════════════════════
# Installa just: cargo install just
# Esegui: just <comando>

# Default: mostra i comandi disponibili
default:
    @just --list

# Setup completo ambiente di sviluppo (una volta)
setup:
    bash scripts/setup.sh

# Build tutto il workspace
build:
    cargo build --workspace

# Build in modalità release
release:
    cargo build --workspace --release

# Esegui tutti i test (Rust + Python)
test:
    cargo test --workspace
    cd python && python -m pytest tests/ -v

# Test solo Rust
test-rust:
    cargo test --workspace

# Test solo Python
test-python:
    cd python && python -m pytest tests/ -v

# Test con output verboso
test-verbose:
    cargo test --workspace -- --nocapture

# Avvia un singolo nodo in modalità sviluppo
dev:
    cargo run --bin varcavia-node -- --config configs/node_default.toml --log-level debug

# Avvia rete locale di N nodi (default 3)
network num="3":
    bash scripts/run_local_network.sh {{num}}

# Lint tutto
lint:
    cargo clippy --workspace -- -D warnings
    cd python && python -m ruff check .

# Formatta tutto il codice
fmt:
    cargo fmt --all
    cd python && python -m ruff format .

# Controlla formattazione senza modificare
fmt-check:
    cargo fmt --all -- --check
    cd python && python -m ruff format --check .

# Benchmark prestazioni
bench:
    cargo bench --workspace

# Pulisci artefatti di build
clean:
    cargo clean
    rm -rf ~/varcavia-data/node-*
    find python -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true

# Controlla che tutto compili (veloce)
check:
    cargo check --workspace

# Genera documentazione
doc:
    cargo doc --workspace --no-deps --open

# Inserisci un dato di test via API (nodo deve essere attivo)
test-insert:
    curl -s -X POST http://localhost:8080/api/v1/data \
        -H "Content-Type: application/json" \
        -d '{"content": "La temperatura a Roma è 22°C", "domain": "climate", "source": "test-sensor-01"}' \
        | jq .

# Query stato nodo via API
test-status:
    curl -s http://localhost:8080/api/v1/node/status | jq .

# Inizializza un nodo pulito
init:
    cargo run --bin varcavia-node -- init --data-dir ~/varcavia-data/node-01
