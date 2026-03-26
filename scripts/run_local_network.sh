#!/usr/bin/env bash
# ════════════════════════════════════════════════════════════
# VARCAVIA — Avvia Rete Locale Multi-Nodo
# ════════════════════════════════════════════════════════════
#
# Uso: bash scripts/run_local_network.sh [NUM_NODI]
# Default: 3 nodi
#   Nodo 1: API :8080, P2P :8180
#   Nodo 2: API :8081, P2P :8181
#   Nodo 3: API :8082, P2P :8182
#
set -euo pipefail

NUM_NODES=${1:-3}
BASE_API_PORT=8080
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

# Build
log "Building varcavia-node..."
cargo build --bin varcavia-node 2>&1 | tail -1 || true
log ""

# Costruisci la lista dei peer P2P per ogni nodo
get_peers() {
    local my_index=$1
    local peers=""
    for ((j=0; j<NUM_NODES; j++)); do
        if [ "$j" -ne "$my_index" ]; then
            local p2p_port=$((BASE_API_PORT + j + 100))
            if [ -n "$peers" ]; then
                peers="$peers,"
            fi
            peers="${peers}127.0.0.1:${p2p_port}"
        fi
    done
    echo "$peers"
}

log "Avvio rete locale con $NUM_NODES nodi..."
log ""

for ((i=0; i<NUM_NODES; i++)); do
    api_port=$((BASE_API_PORT + i))
    p2p_port=$((api_port + 100))
    node_num=$((i + 1))
    node_name=$(printf 'node-%02d' "$node_num")
    data_dir="$HOME/varcavia-data/$node_name"
    peers=$(get_peers "$i")

    mkdir -p "$data_dir"

    log "Nodo $node_name — API :$api_port — P2P :$p2p_port"

    cargo run --bin varcavia-node -- \
        --port "$api_port" \
        --data-dir "$data_dir" \
        --peers "$peers" \
        --log-level info \
        2>&1 | sed "s/^/[$node_name] /" &

    PIDS+=($!)
    sleep 1
done

log ""
log "═══════════════════════════════════════"
log "  Rete locale attiva: $NUM_NODES nodi"
log "  API ports: $BASE_API_PORT-$((BASE_API_PORT + NUM_NODES - 1))"
log "  P2P ports: $((BASE_API_PORT + 100))-$((BASE_API_PORT + NUM_NODES - 1 + 100))"
log ""
log "  Test rapido:"
log "    curl -s -X POST http://localhost:8080/api/v1/data \\"
log "      -H 'Content-Type: application/json' \\"
log "      -d '{\"content\":\"Roma: 22C\",\"domain\":\"climate\",\"source\":\"test\"}'"
log ""
log "  Stato nodi:"
for ((i=0; i<NUM_NODES; i++)); do
    log "    curl -s http://localhost:$((BASE_API_PORT + i))/api/v1/node/status"
done
log ""
log "  Premi Ctrl+C per terminare"
log "═══════════════════════════════════════"

wait
