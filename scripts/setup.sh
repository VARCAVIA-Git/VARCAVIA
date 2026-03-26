#!/usr/bin/env bash
# ════════════════════════════════════════════════════════════
# VARCAVIA — Setup Iniziale Ambiente di Sviluppo
# ════════════════════════════════════════════════════════════
#
# Eseguire una sola volta: bash scripts/setup.sh
# Prerequisiti: Ubuntu 22.04+, connessione internet
#
set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log() { echo -e "${GREEN}[VARCAVIA]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
err() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

log "════════════════════════════════════════════"
log "  VARCAVIA — Setup Ambiente di Sviluppo"
log "════════════════════════════════════════════"

# ── 1. Dipendenze di sistema ──
log "1/7 — Installazione dipendenze di sistema..."
sudo apt-get update -qq
sudo apt-get install -y -qq \
    build-essential \
    pkg-config \
    libssl-dev \
    libclang-dev \
    cmake \
    protobuf-compiler \
    python3 \
    python3-pip \
    python3-venv \
    curl \
    git \
    jq

# ── 2. Rust ──
if command -v rustc &> /dev/null; then
    RUST_VER=$(rustc --version | cut -d' ' -f2)
    log "2/7 — Rust già installato: $RUST_VER"
    rustup update stable
else
    log "2/7 — Installazione Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Verifica versione minima
RUST_VER=$(rustc --version | cut -d' ' -f2)
log "       Rust: $RUST_VER"

# ── 3. just (task runner) ──
if command -v just &> /dev/null; then
    log "3/7 — just già installato"
else
    log "3/7 — Installazione just..."
    cargo install just
fi

# ── 4. Python venv ──
log "4/7 — Setup Python virtual environment..."
VENV_DIR=".venv"
if [ ! -d "$VENV_DIR" ]; then
    python3 -m venv "$VENV_DIR"
fi
source "$VENV_DIR/bin/activate"
pip install --upgrade pip -q
pip install -r python/requirements.txt -q

# ── 5. Directory dati ──
log "5/7 — Creazione directory dati..."
mkdir -p ~/varcavia-data
mkdir -p models

# ── 6. Modello ONNX per embedding ──
MODEL_PATH="models/all-MiniLM-L6-v2.onnx"
if [ -f "$MODEL_PATH" ]; then
    log "6/7 — Modello ONNX già presente"
else
    log "6/7 — Download modello ONNX (all-MiniLM-L6-v2, ~80MB)..."
    ONNX_URL="https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx"
    curl -L -o "$MODEL_PATH" "$ONNX_URL" 2>/dev/null || {
        warn "Download modello fallito. Potrai scaricarlo manualmente dopo."
        warn "URL: $ONNX_URL"
        warn "Destinazione: $MODEL_PATH"
    }
fi

# ── 7. Verifica build ──
log "7/7 — Verifica compilazione workspace..."
cargo check --workspace 2>&1 || {
    warn "La compilazione ha prodotto errori. Questo è normale se è il primo setup."
    warn "Risolvi gli errori e rilancia: cargo check --workspace"
}

log ""
log "════════════════════════════════════════════"
log "  Setup completato!"
log "════════════════════════════════════════════"
log ""
log "  Comandi rapidi:"
log "    just build     — Compila tutto"
log "    just test      — Esegui i test"
log "    just dev       — Avvia un nodo di sviluppo"
log "    just network   — Avvia rete locale (3 nodi)"
log ""
log "  Per attivare il venv Python:"
log "    source .venv/bin/activate"
log ""
