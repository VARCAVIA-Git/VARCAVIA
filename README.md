# \V/ VARCAVIA

**The verification layer for the world's data**

> **Live demo:** https://varcavia-production.up.railway.app
> **Protocol spec:** [docs/VERITPROTOCOL.md](docs/VERITPROTOCOL.md)
> **OpenAPI:** [docs/openapi.yaml](docs/openapi.yaml)
> **MCP for Claude:** [docs/MCP.md](docs/MCP.md)

An open protocol for cryptographically verified data. Every fact gets dual fingerprints, Ed25519 signatures, a 5-tier trust score, and fuzzy verification.

## Quick Start

```bash
# Build and run
cargo run --bin varcavia-node -- --port 8080

# Verify a fact (read-only — never inserts)
curl "http://localhost:8080/api/v1/verify?fact=Earth+has+a+mean+radius+of+6371+kilometres."

# Search the database
curl "http://localhost:8080/api/v1/search?q=Earth+radius&limit=5"

# Submit a new fact
curl -X POST http://localhost:8080/api/v1/data \
  -H 'Content-Type: application/json' \
  -d '{"content":"Roma: 22C","domain":"climate","source":"sensor-01"}'
```

The node auto-seeds with 400+ curated facts on first start (works offline).

## What It Does

**Verify** — Check if a fact exists in the network. Fuzzy matching finds similar facts even with different wording. `/verify` is read-only; it never inserts data.

**Trust** — Every fact has a 5-tier trust level (T0-T4) based on attestations from independent sources, with authority weighting and contradiction detection.

**Certify** — Facts get a Data DNA: Ed25519 signature, BLAKE3+SHA3-512 dual fingerprint, microsecond timestamp, custody chain.

## Architecture

```
┌─────────────────────┐
│  Trust Tier System   │  ← T0-T4 with authority scoring
├─────────────────────┤
│  REST API + MCP      │  ← 24 endpoints + Claude integration
├─────────────────────┤
│  Clean Data Engine   │  ← 6-stage pipeline: dedup, validate, score
├─────────────────────┤
│  ARC Consensus       │  ← Distributed validation < 5s
├─────────────────────┤
│  Data DNA            │  ← Cryptographic identity layer
├─────────────────────┤
│  Transport (VTP)     │  ← P2P messaging + CRDT sync
└─────────────────────┘
```

### Crates (8)

| Crate | Purpose |
|-------|---------|
| `ddna` | Data DNA — Ed25519 signatures, BLAKE3+SHA3 fingerprints, custody chain |
| `vtp` | Transport Protocol — packets, priority queuing, compression, CRDT sync |
| `arc` | Adaptive Resonance Consensus — committee selection, voting, reputation |
| `cde` | Clean Data Engine — 6-stage pipeline: dedup, validate, normalize, score |
| `uag` | Universal Access Gateway — Axum HTTP, REST API, trust tiers, translator |
| `node` | Node binary — storage (sled), P2P networking, auto-seed, background crawler |
| `crawler` | Fact crawler — Wikipedia, Wikidata SPARQL, 400+ hardcoded facts |
| `mcp` | MCP server — 4 tools for Claude: verify_fact, search_facts, submit_fact, get_stats |

### Trust Tier System

| Tier | Label | Requirements |
|------|-------|-------------|
| T0 | Unattested | No attestations |
| T1 | Attested | 1+ attestation, authority >= 1 |
| T2 | Corroborated | 2+ domains, authority >= 5 |
| T3 | Authoritative | Authority >= 15, 1+ institutional/peer-reviewed source |
| T4 | Canonical | Authority >= 50, 2+ high-authority, age > 7 days, 0 contradictions |

Source weights: Institutional=10, PeerReviewed=5, Media=3, Website=1, Anonymous=0.5

## API Reference

### Verification (read-only)
```
GET  /api/v1/verify?fact=...         Fuzzy verify: exact hash → 70%+ trigram → similar → not_found
GET  /api/v1/search?q=...&limit=5    Semantic search by trigram similarity
```

### Data Operations
```
POST   /api/v1/data                  Insert fact (dDNA + CDE pipeline + trust T0)
GET    /api/v1/data/:id              Get fact by ID
GET    /api/v1/data/:id/dna          Full Data DNA certificate
GET    /api/v1/data/:id/trust        Full TrustRecord
POST   /api/v1/attest/:id            Add attestation (promotes trust tier)
POST   /api/v1/extract               Extract claims from long text
```

### Batch (Enterprise)
```
POST   /api/v1/batch/verify          Verify array of facts: {"facts":["...", "..."]}
POST   /api/v1/batch/submit          Insert array: {"items":[{"content":"...","domain":"..."}]}
```

### System
```
GET    /api/v1/node/status           Node info + avg_latency_ms
GET    /api/v1/metrics               claims_per_second, facts_ingested, storage_bytes
GET    /api/v1/stats/tiers           Trust tier distribution: {"T0":0,"T1":400,...}
GET    /health                       Health check
```

### API Key Authentication
Set `VARCAVIA_API_KEY` env var to require `X-API-Key` header for POST endpoints. GET endpoints are always public.

## MCP Server (Claude Integration)

```json
{
  "mcpServers": {
    "varcavia": {
      "command": "path/to/varcavia-mcp",
      "env": { "VARCAVIA_URL": "https://varcavia-production.up.railway.app" }
    }
  }
}
```

Tools: `verify_fact`, `search_facts`, `submit_fact`, `get_stats`. See [docs/MCP.md](docs/MCP.md).

## Docker

```bash
docker build -t varcavia .
docker run -p 8080:8080 varcavia
```

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Core | Rust 1.78+, 8-crate Cargo workspace |
| Crypto | Ed25519, BLAKE3, SHA3-512 |
| Storage | sled (embedded KV) |
| API | Axum 0.7 (async) |
| Consensus | Custom ARC protocol |
| Trust | 5-tier VERIT system |
| Crawler | Wikipedia + Wikidata SPARQL |
| MCP | JSON-RPC over stdio |

## Stats

- **224 tests** across 8 crates
- **10,124 lines** of Rust
- **400+ curated facts** (offline seed)
- **24 API endpoints**
- **<10ms** average latency

## License

AGPL-3.0
