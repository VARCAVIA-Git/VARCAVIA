# \V/ VARCAVIA

**Verifiable Autonomous Registry for Clean, Accessible, Validated & Interlinked Archives**

> **Live demo:** https://varcavia-production.up.railway.app
> **Protocol spec:** [docs/VERITPROTOCOL.md](docs/VERITPROTOCOL.md)

Decentralized infrastructure where every piece of data is cryptographically verified, deduplicated, and scored for reliability — automatically.

## Quick Start

```bash
# Build
cargo build --bin varcavia-node

# Run a node
cargo run --bin varcavia-node -- --port 8080

# Verify a fact
curl "http://localhost:8080/api/v1/verify?fact=Earth+diameter+is+12742+km"
```

## What It Does

Every fact that enters VARCAVIA gets a **Data DNA** — a cryptographic identity that proves:

- **Who** created it (Ed25519 signature)
- **What** it contains (BLAKE3 + SHA3-512 dual fingerprint)
- **When** it was created (microsecond-precision timestamp)
- **How reliable** it is (composite score from 6-stage pipeline)

Duplicate or tampered data is automatically detected and rejected.

## Architecture

```
                    ┌─────────────────────┐
                    │    REST / GraphQL    │  ← Universal Access Gateway
                    ├─────────────────────┤
                    │  Clean Data Engine   │  ← 6-stage purification pipeline
                    ├─────────────────────┤
                    │   ARC Consensus      │  ← Distributed validation < 200ms
                    ├─────────────────────┤
                    │     Data DNA         │  ← Cryptographic identity layer
                    ├─────────────────────┤
                    │  Transport (VTP)     │  ← P2P messaging + CRDT sync
                    └─────────────────────┘
```

### Crates

| Crate | Purpose |
|-------|---------|
| `ddna` | Data DNA — Ed25519 signatures, BLAKE3+SHA3 fingerprints, custody chain |
| `vtp` | Transport Protocol — packets, priority queuing, compression, CRDT sync |
| `arc` | Adaptive Resonance Consensus — committee selection, voting, reputation |
| `cde` | Clean Data Engine — 6-stage pipeline: dedup, validate, normalize, score |
| `uag` | Universal Access Gateway — Axum HTTP server, REST API, format translator |
| `node` | Node binary — wires everything together, storage (sled), P2P networking |

### CDE Pipeline (6 Stages)

```
Input → [Hash Dedup] → [LSH Near-Dedup] → [Semantic Dedup] → [Source Validation] → [Normalization] → [Scoring] → Output
         BLAKE3 O(1)    MinHash O(1)       Trigram Jaccard    Ed25519 verify        VUF + zstd        Composite
```

## API Reference

### Hero Endpoint

```
GET /api/v1/verify?fact=Earth+diameter+is+12742+km
```

Returns Data DNA + reliability score for any fact. This is the main demo endpoint.

### Data Operations

```
POST   /api/v1/data              Insert data (creates dDNA, runs CDE pipeline)
GET    /api/v1/data/:id          Get data by ID (blake3 hex)
GET    /api/v1/data/:id/dna      Get full Data DNA
GET    /api/v1/data/:id/score    Get reliability score
DELETE /api/v1/data/:id          Soft delete
POST   /api/v1/data/query        Query by domain: {"query":"", "domain":"climate"}
POST   /api/v1/data/verify       Verify content: {"id":"...", "content":"..."}
```

### Node & Network

```
GET    /api/v1/node/status          Node info (uptime, data count, node ID)
GET    /api/v1/node/peers           Connected peers
GET    /api/v1/node/stats           Statistics
GET    /api/v1/node/consensus/:id   Consensus state for a data item
GET    /api/v1/network/health       Network health
POST   /api/v1/translate            Format conversion: {"data":..., "from_format":"json", "to_format":"xml"}
```

### Insert Example

```bash
curl -X POST http://localhost:8080/api/v1/data \
  -H 'Content-Type: application/json' \
  -d '{"content":"Roma: 22°C","domain":"climate","source":"sensor-01"}'
```

Response:
```json
{
  "id": "dcd380ce7fb6b778c7ccba044497321a079f1f57d237d00189877bf66f2867cc",
  "status": "accepted",
  "score": 0.73
}
```

## Multi-Node Network

```bash
# Start a 3-node local network
bash scripts/run_local_network.sh 3

# Nodes auto-discover peers and replicate data via ARC consensus
```

When data is inserted on any node:
1. The node creates a dDNA and runs the CDE pipeline
2. It sends a `VoteRequest` to all peers
3. Peers validate (Ed25519, fingerprint, timestamp) and vote
4. If consensus score >= 0.67: data is confirmed and replicated

## Docker

```bash
docker build -t varcavia .
docker run -p 8080:8080 varcavia
```

## Web Dashboard

```bash
cd web/dashboard
npm install
npm run dev
# Open http://localhost:5173
```

Real-time monitoring: node status, data table, peer list, insert form.

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Core | Rust 1.78+, Cargo workspace |
| Crypto | Ed25519 (ed25519-dalek), BLAKE3, SHA3-512 |
| Storage | sled (embedded KV store) |
| API | Axum (async HTTP) |
| Consensus | Custom ARC protocol |
| Compression | zstd |
| Dashboard | React + Vite |

## Contributing

```bash
# Setup
cargo build --workspace

# Test
cargo test --workspace

# Lint
cargo clippy --workspace

# Format
cargo fmt --all
```

All contributions must pass `cargo test` and `cargo clippy` with zero errors.

## License

AGPL-3.0 — Modifications must remain open source.
