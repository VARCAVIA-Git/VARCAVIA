# VARCAVIA v2 — Comprehensive Audit Report

**Date:** 2026-03-27
**Commit:** 869021a
**Auditor:** Claude Opus 4.6

## A. General State

VARCAVIA is a well-structured, functional Rust monorepo with 8 crates, 224 tests (all passing), a production deployment on Railway, and a polished landing page. The system verifies facts via cryptographic fingerprinting (BLAKE3+SHA3), trigram similarity search, a 5-tier trust system, and an MCP server for Claude integration. The DB auto-seeds with 400+ curated facts on every deploy. The codebase is clean, well-organized, and production-ready for a v0.1 launch.

## B. Numbers

| Metric | Value |
|--------|-------|
| Crates | 8 (arc, cde, crawler, ddna, mcp, node, uag, vtp) |
| Total Rust LOC | 10,124 |
| Total HTML/JS LOC | 561 |
| Test count | 224 (arc:15, cde:43, crawler:15, ddna:35, mcp:11, node:13, uag:78, vtp:14) |
| API endpoints | 24 |
| Hardcoded seed facts | 400 |
| Total seed facts (with Wikipedia fallbacks) | 488+ |
| Production facts (with Wikidata) | 482-1484 (varies by deploy) |
| Commits | 30 on main |

## C. Critical Problems (Block Launch)

**None.** All 224 tests pass. Build succeeds. Production is live and functional.

## D. Moderate Problems (Fix Soon)

1. **Clippy error in trust.rs:202** — `needless_range_loop` on the independence detection loop. Blocks `cargo clippy` from passing cleanly.

2. **.gitignore incomplete** — Missing entries for `*.db`, `node_modules/`, `sled-data/`, `.env`. The `.env` file is tracked in git (contains no secrets, only dev defaults, but should be in gitignore for hygiene).

3. **node/src/storage.rs is dead code** — Entire `Storage` struct and all its methods are unused (the node uses `uag/state.rs` AppState instead). 17 dead_code warnings from this.

4. **node/src/config.rs is dead code** — `NodeConfig` and all section structs are never constructed. Config is loaded via CLI args, not TOML.

5. **README.md is outdated** — Describes v1 features (basic verify, landing page). Doesn't mention: Trust Tiers, MCP server, batch API, Wikidata crawler, fuzzy verify, API key auth, 400+ facts.

6. **CLAUDE.md is outdated** — Still describes Phase 1-6 development plan. Doesn't reflect current architecture (trust system, crawler, MCP, auto-seed).

## E. Minor Problems (Tech Debt)

1. **9 TODO comments** — All are future-phase features (ONNX embeddings, libp2p, GraphQL, delta compression). Not blocking.

2. **11 unwrap() in production code** — All safe: Mutex locks (5), literal string parse (1), serde_json::to_string on known-good data (2), partial_cmp on non-NaN floats (2), reqwest client build (1).

3. **graphql.rs is a stub** — 76 lines, returns error. Placeholder for future Phase 6. Not exposed via routes.

4. **network/topology endpoint returns empty** — `{"nodes":[],"edges":[]}` always. Not used by landing page.

5. **Dockerfile.manual exists** alongside Dockerfile — Likely an older version. Should be removed or documented.

6. **CONSIGLI/ directory** — 6 Italian-language advice files. Internal development notes, not user-facing.

7. **sdk/ directory** — Contains curl/python/javascript examples. Not tested, may be outdated.

## F. Inconsistencies

1. **CLAUDE.md says "crates/uag/src/rest.rs"** for REST endpoints but doesn't list trust.rs, trust routes, batch routes, or search routes.

2. **CLAUDE.md says "Fase 1-6"** development plan — project is well past Fase 6.

3. **README says "1000 req/s"** throughput target — actual measured avg_latency is 3-10ms which is ~100-300 req/s.

4. **openapi.yaml** doesn't include `/api/v1/attest/:id`, `/api/v1/data/:id/trust`, `/api/v1/stats/tiers`.

5. **Landing page says "7 domains"** hardcoded — actual domains are: geography, science, history, technology, health, climate, politics, general (8).

## G. Missing or Obsolete Files

| File | Status |
|------|--------|
| `Dockerfile.manual` | Obsolete — duplicate of Dockerfile |
| `scripts/setup.sh` | References old structure, needs update |
| `scripts/seed_test_data.py` | Python seeder, superseded by Rust auto-seed |
| `python/` directory | Entire Python agent system — not integrated with current Rust-only architecture |
| `proto/` directory | Protobuf definitions — not used (using serde JSON/MessagePack instead) |
| `configs/node_default.toml` | Not loaded by current code (CLI args used instead) |
| `tests/` directory | Missing — no integration test directory (all tests are unit tests inline) |
| `web/dashboard/` | React dashboard — separate from landing page, not built or served |

## H. Recommendations (Priority Order)

1. **Fix clippy error** in trust.rs independence loop
2. **Update .gitignore** with missing entries
3. **Remove dead code** in node/storage.rs and node/config.rs (or keep with `#[allow(dead_code)]`)
4. **Update README.md** to reflect v2 features
5. **Update openapi.yaml** with trust/attest endpoints
6. **Fix landing page** domain count (7 -> 8)
7. **Remove Dockerfile.manual** if not needed
8. **Add integration tests** in tests/ directory for multi-crate flows
9. **Consider removing** python/, proto/, CONSIGLI/ if not planned for near future
10. **Document** the trust tier system in docs/ARCHITECTURE.md
