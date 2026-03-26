#!/usr/bin/env bash
# ════════════════════════════════════════════════════════════
# VARCAVIA — Avvia Rete Locale Multi-Nodo
# ════════════════════════════════════════════════════════════
#
# Uso: bash scripts/run_local_network.sh [NUM_NODI]
# Default: 3 nodi sulle porte 7700-7702, API su 8080-8082
#
set -euo pipefail

NUM_NODES=${1:-3}
BASE_PORT=7700
API_BASE_PORT=8080
BASE_DATA_DIR="$HOME/varcavia-data"
PIDS=()

GREEN='\033[0;32m'
NC='\033[0m'
log() { echo -e "${GREEN}[VARCAVIA]${NC} $1"; }

cleanup() {
    log "Shutdown rete locale..."
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    wait 2>/dev/null || true
    log "Tutti i nodi terminati."
}
trap cleanup EXIT INT TERM

log "Avvio rete locale con $NUM_NODES nodi..."
log ""

# Build first
cargo build --bin varcavia-node --release 2>&1 | tail -1

for i in $(seq 0 $((NUM_NODES - 1))); do
    PORT=$((BASE_PORT + i))
    API_PORT=$((API_BASE_PORT + i))
    NODE_NAME="varcavia-dev-$(printf '%02d' $((i + 1)))"
    DATA_DIR="$BASE_DATA_DIR/node-$(printf '%02d' $((i + 1)))"
    
    mkdir -p "$DATA_DIR"
    
    # Genera config dinamica per ogni nodo
    CONFIG_FILE="/tmp/varcavia-node-${i}.toml"
    cat > "$CONFIG_FILE" << EOF
[node]
name = "$NODE_NAME"
data_dir = "$DATA_DIR"
log_level = "info"

[network]
listen_addr = "127.0.0.1"
listen_port = $PORT
bootstrap_nodes = [$(if [ $i -gt 0 ]; then echo "\"127.0.0.1:$BASE_PORT\""; fi)]
max_peers = 50
mdns_enabled = true

[storage]
engine = "rocksdb"
max_size_gb = 5
compression = "zstd"
cache_size_mb = 128

[arc]
committee_size = 3
confirmation_threshold = 0.67
validation_timeout_ms = 500
reputation_decay_rate = 0.01

[cde]
dedup_lsh_threshold = 0.85
semantic_dedup_threshold = 0.1
freshness_window_hours = 24
min_source_reputation = 0.3

[ai]
onnx_model_path = "models/all-MiniLM-L6-v2.onnx"
embedding_dimensions = 384
max_batch_size = 32
agent_check_interval_secs = 10

[api]
enabled = true
bind_addr = "127.0.0.1:$API_PORT"
cors_origins = ["http://localhost:5173"]
rate_limit_per_sec = 100
EOF
    
    log "Nodo $NODE_NAME — P2P :$PORT — API :$API_PORT — Data: $DATA_DIR"
    
    # Avvia nodo in background
    ./target/release/varcavia-node --config "$CONFIG_FILE" &
    PIDS+=($!)
done

log ""
log "═══════════════════════════════════════"
log "  Rete locale attiva: $NUM_NODES nodi"
log "  P2P ports: $BASE_PORT-$((BASE_PORT + NUM_NODES - 1))"
log "  API ports: $API_BASE_PORT-$((API_BASE_PORT + NUM_NODES - 1))"
log "  Premi Ctrl+C per terminare"
log "═══════════════════════════════════════"

# Attendi che tutti i processi terminino
wait
