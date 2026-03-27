# \V/ VARCAVIA

### The verification layer for the world's data

Open protocol for cryptographically verified data. Every fact gets a **Data DNA** — a tamper-proof certificate with source identity, timestamp, trust tier, and cryptographic proof.

**Live demo:** https://varcavia-production.up.railway.app
**OpenAPI spec:** [docs/openapi.yaml](docs/openapi.yaml)
**MCP for Claude:** [docs/MCP.md](docs/MCP.md)
**Protocol spec:** [docs/VERITPROTOCOL.md](docs/VERITPROTOCOL.md)

## Try it

```bash
# Verify a fact (read-only — never inserts data)
curl "https://varcavia-production.up.railway.app/api/v1/verify?fact=Earth+has+a+mean+radius+of+6371+kilometres."

# Search the database
curl "https://varcavia-production.up.railway.app/api/v1/search?q=speed+of+light&limit=5"

# Get trust tier distribution
curl "https://varcavia-production.up.railway.app/api/v1/stats/tiers"

# Submit a new fact
curl -X POST "https://varcavia-production.up.railway.app/api/v1/data" \
  -H "Content-Type: application/json" \
  -d '{"content":"The Moon orbits Earth at 384400 km","domain":"science","source":"textbook"}'

# Batch verify
curl -X POST "https://varcavia-production.up.railway.app/api/v1/batch/verify" \
  -H "Content-Type: application/json" \
  -d '{"facts":["Earth diameter is 12742 km","Water boils at 100 degrees Celsius"]}'
```

## Architecture

Pure Rust. 8 crates. ~10K LOC. 246 tests. Zero `unsafe`.

```
┌─────────────────────────────┐
│  Trust Tier System (T0-T4)  │  Authority scoring, independence detection
├─────────────────────────────┤
│  REST API (24 endpoints)    │  Axum + MCP server for Claude
├─────────────────────────────┤
│  Clean Data Engine          │  6-stage: dedup → validate → normalize → score
├─────────────────────────────┤
│  ARC Consensus              │  Reputation-weighted distributed validation
├─────────────────────────────┤
│  Data DNA                   │  Ed25519 + BLAKE3 + SHA3-512 fingerprints
├─────────────────────────────┤
│  Transport Protocol (VTP)   │  TCP P2P, CRDT sync, priority queuing
└─────────────────────────────┘
```

| Crate | Purpose |
|-------|---------|
| `ddna` | Data DNA — Ed25519 signing, BLAKE3+SHA3-512 dual fingerprint, custody chain |
| `vtp` | Transport — TCP messages, priority queuing, zstd compression, CRDT sync |
| `arc` | Consensus — committee selection, reputation-weighted voting |
| `cde` | Clean Data Engine — hash dedup, LSH near-dedup, trigram semantic dedup, scoring |
| `uag` | API Gateway — 24 Axum REST endpoints, trust tiers, keyword matching, format translator |
| `node` | Binary — sled storage, TCP networking, auto-seed, background Wikidata crawler |
| `crawler` | Facts — Wikipedia parser, Wikidata SPARQL, 400+ hardcoded curated facts |
| `mcp` | MCP server — 4 tools for Claude: verify_fact, search_facts, submit_fact, get_stats |

## Trust Tier System

Facts progress through 5 trust levels based on independent attestations:

| Tier | Label | Requirements |
|------|-------|-------------|
| **T0** | Unattested | No attestations |
| **T1** | Attested | 1+ source, authority weight >= 1 |
| **T2** | Corroborated | 2+ independent domains, authority >= 5 |
| **T3** | Authoritative | Authority >= 15, 1+ institutional/peer-reviewed source |
| **T4** | Canonical | Authority >= 50, 2+ high-authority, age > 7 days, 0 contradictions |

Source weights: **Institutional** = 10, **PeerReviewed** = 5, **Media** = 3, **Website** = 1, **Anonymous** = 0.5

Independence scoring: same-domain attestations count 0.3, cross-domain count 1.0.

## API Endpoints

### Verification (read-only)
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/verify?fact=...` | Fuzzy verify: exact hash → keyword+number match → trigram fallback |
| GET | `/api/v1/search?q=...&limit=5` | Semantic search by trigram similarity |

### Data
| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/data` | Submit a fact (runs CDE pipeline) |
| GET | `/api/v1/data/:id` | Get fact by ID |
| GET | `/api/v1/data/:id/dna` | Full Data DNA certificate |
| GET | `/api/v1/data/:id/trust` | Full TrustRecord with attestations |
| POST | `/api/v1/attest/:id` | Add attestation (promotes trust tier) |
| POST | `/api/v1/extract` | Extract claims from long text |

### Batch (Enterprise)
| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/batch/verify` | `{"facts":["..."]}` — verify array |
| POST | `/api/v1/batch/submit` | `{"items":[{...}]}` — insert array |

### System
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/node/status` | Node info + avg_latency_ms |
| GET | `/api/v1/metrics` | Operational metrics |
| GET | `/api/v1/stats/tiers` | Trust tier distribution |
| GET | `/health` | Health check |

API key: set `VARCAVIA_API_KEY` env var to require `X-API-Key` header for POST endpoints. GET is always public.

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

4 tools: `verify_fact`, `search_facts`, `submit_fact`, `get_stats`. See [docs/MCP.md](docs/MCP.md).

## Building

```bash
cargo build --workspace        # Build all 8 crates
cargo test --workspace         # Run 246 tests
cargo clippy --workspace       # Lint (0 warnings)
cargo run --bin varcavia-node   # Start a node on :8080
```

## Docker

```bash
docker build -t varcavia .
docker run -p 8080:8080 varcavia
```

## Status

**v0.1** — Production deployment on Railway. All 24 endpoints verified. ~17ms server-side latency.

- 400+ curated seed facts (auto-seeded on deploy)
- 5-tier trust system with authority scoring and independence detection
- Keyword extraction + number normalization matching in `/verify`
- Wikidata SPARQL crawler (background, every 6h)
- MCP server for Claude integration
- Batch API for enterprise use

## Stats

- **246 tests** across 8 crates
- **~10K lines** of Rust
- **24 API endpoints** (all production-verified)
- **~17ms** average server-side latency
- **0** clippy warnings

## License

- **Code:** AGPL-3.0
- **Protocol spec:** CC BY-SA 4.0
