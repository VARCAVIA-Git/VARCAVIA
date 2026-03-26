#!/usr/bin/env bash
# VARCAVIA — Benchmark Prestazioni
set -euo pipefail

echo "═══════════════════════════════════════"
echo "  VARCAVIA — Benchmark Suite"
echo "═══════════════════════════════════════"

echo ""
echo "1. dDNA Creation Benchmark..."
cargo bench -p varcavia-ddna 2>/dev/null || echo "   (non ancora implementato)"

echo ""
echo "2. Compression Benchmark..."
cargo bench -p varcavia-vtp 2>/dev/null || echo "   (non ancora implementato)"

echo ""
echo "3. ARC Consensus Simulation..."
cargo bench -p varcavia-arc 2>/dev/null || echo "   (non ancora implementato)"

echo ""
echo "4. API Throughput Test..."
if curl -s http://localhost:8080/api/v1/node/status > /dev/null 2>&1; then
    echo "   Nodo attivo, esecuzione test..."
    # Semplice test di throughput
    START=$(date +%s%N)
    for i in $(seq 1 100); do
        curl -s http://localhost:8080/api/v1/node/status > /dev/null
    done
    END=$(date +%s%N)
    ELAPSED=$(( (END - START) / 1000000 ))
    RPS=$(( 100 * 1000 / ELAPSED ))
    echo "   100 richieste in ${ELAPSED}ms = ${RPS} req/s"
else
    echo "   Nodo non attivo — avvia con: just dev"
fi

echo ""
echo "═══════════════════════════════════════"
echo "  Benchmark completato"
echo "═══════════════════════════════════════"
