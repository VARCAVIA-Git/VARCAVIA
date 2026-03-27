# VARCAVIA — Development Instructions

## What is VARCAVIA

**Verifiable Autonomous Registry for Clean, Accessible, Validated & Interlinked Archives**

Open protocol for cryptographically verified data. Pure Rust monorepo, 8 crates, ~10K LOC, 246 tests.

**Production:** https://varcavia-production.up.railway.app
**Repo:** git@github.com:VARCAVIA-Git/VARCAVIA.git

## Current Architecture (v2)

### Crates

| Crate | Files | Purpose |
|-------|-------|---------|
| `crates/ddna` | 8 source files | Data DNA — Ed25519 signatures, BLAKE3+SHA3-512 fingerprints, custody chain, MessagePack codec |
| `crates/vtp` | 8 source files | Transport Protocol — TCP messages, priority queuing, zstd compression, CRDT sync |
| `crates/arc` | 6 source files | Adaptive Resonance Consensus — committee selection, reputation-weighted voting |
| `crates/cde` | 7 source files | Clean Data Engine — 6-stage pipeline: hash dedup, LSH, trigram semantic dedup, validation, normalization, scoring |
| `crates/uag` | 10 source files | Universal Access Gateway — Axum REST API (24 endpoints), trust tiers, keyword matching, format translator, middleware |
| `crates/node` | 5 source files | Node binary — sled storage, TCP P2P networking, auto-seed, background Wikidata crawler |
| `crates/crawler` | 2 source files | Facts crawler — Wikipedia HTML parser, Wikidata SPARQL, 400+ hardcoded curated facts |
| `crates/mcp` | 1 source file | MCP server — JSON-RPC over stdio, 4 tools for Claude integration |

### Key Files

- `crates/uag/src/rest.rs` — All 24 REST endpoint handlers (~1950 lines)
- `crates/uag/src/keyword_match.rs` — Keyword extraction, number normalization, unit matching (~470 lines)
- `crates/uag/src/trust.rs` — Trust Tier System with authority scoring (~560 lines)
- `crates/crawler/src/lib.rs` — 400+ hardcoded seed facts + Wikipedia crawler (~910 lines)
- `crates/crawler/src/wikidata.rs` — Wikidata SPARQL queries (~370 lines)
- `crates/node/src/main.rs` — Node bootstrap, auto-seed, background crawler (~420 lines)
- `crates/node/src/network.rs` — TCP P2P with VoteRequest/VoteResponse (~635 lines)
- `web/public/index.html` — Landing page with demo, search, trust tiers (~560 lines)

## Trust Tier System

```
T0 Unattested  — no attestations
T1 Attested    — 1+ attestation, authority_score >= 1
T2 Corroborated — 2+ domains, authority_score >= 5, 2+ attestations
T3 Authoritative — authority_score >= 15, 1+ Institutional or PeerReviewed
T4 Canonical   — authority_score >= 50, 2+ high-authority, age > 7 days, 0 contradictions
```

Source weights: Institutional=10, PeerReviewed=5, MainstreamMedia=3, Website=1, Anonymous=0.5

Independence scoring: same-domain pairs = 0.3, cross-domain = 1.0

All 400+ seed facts start at T1 (PeerReviewed attestation from the node).

## API Endpoints (24 total)

### Verification (read-only)
```
GET  /api/v1/verify?fact=...         Fuzzy verify (hash → trigram ≥70% → similar ≥40% → not_found)
GET  /api/v1/search?q=...&limit=5    Trigram similarity search
```

### Data CRUD
```
POST   /api/v1/data                  Insert fact (dDNA + CDE + trust T0)
GET    /api/v1/data/:id              Get fact by ID
GET    /api/v1/data/:id/dna          Data DNA certificate
GET    /api/v1/data/:id/score        Reliability score
GET    /api/v1/data/:id/trust        Full TrustRecord
DELETE /api/v1/data/:id              Soft delete
POST   /api/v1/data/query            Query by domain
POST   /api/v1/data/verify           Verify content integrity
POST   /api/v1/attest/:id            Add attestation → recompute tier
POST   /api/v1/extract               Extract claims from long text
```

### Batch (Enterprise)
```
POST   /api/v1/batch/verify          {"facts":["...", "..."]}
POST   /api/v1/batch/submit          {"items":[{"content":"...","domain":"..."}]}
```

### System
```
GET    /api/v1/node/status           Node info + avg_latency_ms
GET    /api/v1/node/peers            Connected P2P peers
GET    /api/v1/node/stats            Statistics
GET    /api/v1/node/consensus/:id    Consensus vote history
GET    /api/v1/network/health        Network health
GET    /api/v1/network/topology      Network graph (stub)
GET    /api/v1/metrics               Operational metrics
GET    /api/v1/stats                 Public stats
GET    /api/v1/stats/tiers           Trust tier distribution
POST   /api/v1/translate             Format conversion (JSON↔CSV↔XML)
GET    /health                       Health check
```

### Hero
```
GET    /                             Landing page (include_str index.html)
```

## How /verify Works

1. Compute BLAKE3 hash of the query fact
2. Check for exact hash match in sled DB (prefix `d:`)
3. If found: return "verified" with dDNA, trust info, verification mining (query_count++)
4. If not found: **keyword extraction + number normalization matching** (keyword_match.rs):
   - Extract content keywords (strip stop words, normalize to lowercase)
   - Extract numbers (handle commas: "299,792,458" → 299792458, written multipliers: "14 million" → 14000000)
   - Normalize units (kilometres→km, metres per second→m/s, degrees celsius→celsius)
   - Compute keyword overlap (Jaccard) + number match (1% tolerance)
   - If keyword overlap ≥ 0.6 AND numbers match: return "verified" via keyword match
   - Early termination at overlap ≥ 0.8 (skip remaining facts)
5. Trigram similarity fallback:
   - If best match ≥ 70%: return "verified" via fuzzy match
   - If best match 40-69%: return "similar_found" with related facts
6. Keyword similar_found fallback (overlap ≥ 0.4 + numbers match)
7. If nothing matched: return "not_found"

/verify NEVER inserts data. Only POST /api/v1/data inserts.

## Auto-Seed

On every node startup:
1. Scan DB, remove any data NOT in the 400+ hardcoded seed facts
2. Insert any missing seed facts with CDE pipeline + TrustRecord T1
3. Background crawler starts after 5 minutes:
   - Wikipedia HTML crawl (15 pages)
   - Wikidata SPARQL (countries, elements, planets, people)
   - Repeats every 6 hours
   - Existing facts get additional attestation (tier promotion)
   - New facts get T1 TrustRecord

## Authentication

- `VARCAVIA_API_KEY` env var: if set, POST/PUT/DELETE require `X-API-Key` header
- GET endpoints are always public
- If not set, everything is open

## Deploy

- **Railway:** auto-deploys from main branch
- **Dockerfile:** multi-stage (rust:latest builder → debian:bookworm-slim runtime)
- **Port:** reads `PORT` env var, falls back to `--port` arg, default 8080
- **Bind:** 0.0.0.0 (required for Railway/Docker)
- **Data:** sled DB in `--data-dir` (ephemeral on Railway, auto-seeds on restart)

## Brand Colors

| Name | Hex | Usage |
|------|-----|-------|
| Petrolio | #28516D | Borders, secondary elements |
| Ocra | #D4A11F | Highlights, T1 badge, accents |
| Ciano | #1FA1D4 | Primary action color, links, T2 badge |
| Background | #0A0F14 | Page background |
| Surface | #111A26 | Card backgrounds |

Font: IBM Plex Mono (Google Fonts)

## Conventions

### Rust
- Edition 2021, MSRV 1.78
- `#![deny(clippy::all)]` in library crates
- Errors: `thiserror` for libraries, `anyhow` for binaries
- Async: `tokio` runtime
- Serialization: `serde` + `serde_json` + `rmp-serde`
- Logging: `tracing`
- Every module has inline `#[cfg(test)] mod tests`

### Commit Messages
- Prefixes: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `audit:`
- Tests must pass before every commit

### Commands
```bash
cargo build --workspace          # Build all
cargo test --workspace           # Run 246 tests
cargo clippy --workspace         # Lint (must be 0 warnings)
cargo run --bin varcavia-node    # Run node on :8080
cargo run --bin varcavia-mcp     # Run MCP server (stdio)
cargo run --bin varcavia-node -- seed --port 8080  # Seed via HTTP
```

## Production Stats

- All 24 endpoints verified on production (2026-03-27)
- Server-side latency: ~17ms average
- 246 tests, 0 clippy warnings
- 35+ commits on main

## Known Issues

1. All seed facts are T1 — need external attestations to reach T2+
2. Wikidata crawler may get 403 on some environments (falls back to hardcoded)
3. Landing page hardcodes "8 domains" — should query API
4. `network/topology` endpoint returns empty stub
5. `graphql.rs` is a placeholder stub
