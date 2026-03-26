# The VERIT Protocol

**VARCAVIA Extensible Registry for Immutable Trust**

*Protocol Specification v1.0 — March 2026*

---

## Abstract

The VERIT Protocol defines a decentralized system for cryptographic verification, deduplication, and reliability scoring of arbitrary data. Every datum that enters the network receives a **Data DNA (dDNA)** — a multi-layer cryptographic identity that binds content, source, time, and custody into a single verifiable structure. Nodes reach agreement on data validity through **Adaptive Resonance Consensus (ARC)**, a sub-200ms Byzantine fault-tolerant protocol that replaces energy-intensive mining with reputation-weighted committee voting. A six-stage **Clean Data Engine (CDE)** ensures that only unique, valid, and properly sourced data persists in the network.

The protocol is designed to run on commodity hardware (8 GB RAM, no GPU) and operates fully offline, with peer synchronization when connectivity is available.

---

## 1. Problem Statement

Global data production exceeds 400 exabytes per day. Over 80% of this data is duplicated, corrupted, outdated, or unverifiable. The cost of data quality failures — reconciliation, bad decisions, compliance violations — exceeds $3 trillion annually.

Current approaches fail because they are:

- **Centralized**: single points of trust and failure
- **Post-hoc**: data is cleaned after the fact, not at the point of creation
- **Non-cryptographic**: no mathematical guarantee of provenance or integrity
- **Siloed**: incompatible formats and schemas across domains

VERIT addresses these failures by making data cleanliness a **protocol-level property**: data is verified, certified, and deduplicated *before* it enters the network, not after.

---

## 2. Design Principles

| Principle | Implementation |
|-----------|---------------|
| **Verify at ingestion** | Every datum gets a dDNA at the point of entry, not retroactively |
| **Cryptographic proof** | Ed25519 signatures + BLAKE3/SHA3 dual fingerprints = mathematical certainty |
| **Offline-first** | Nodes function independently; synchronization is opportunistic |
| **No mining, no staking** | ARC consensus uses reputation-weighted committees, not energy waste |
| **Hardware-humble** | Full node on a laptop with 8 GB RAM, no GPU required |
| **Format-agnostic** | JSON, CSV, XML, binary — the protocol handles all via VUF normalization |
| **Defense in depth** | Dual hashing, dual deduplication (exact + semantic), multi-node validation |

---

## 3. Data DNA (dDNA)

The Data DNA is the core identity structure attached to every datum in the network.

### 3.1 Structure

| Field | Type | Size | Description |
|-------|------|------|-------------|
| `version` | `u8` | 1 B | Protocol version (currently `1`) |
| `fingerprint.blake3` | `[u8; 32]` | 32 B | BLAKE3 hash — fast lookup and deduplication |
| `fingerprint.sha3_512` | `[u8; 64]` | 64 B | SHA3-512 hash — cryptographic verification |
| `fingerprint.content_size` | `u64` | 8 B | Original content size in bytes |
| `source.public_key` | `[u8; 32]` | 32 B | Ed25519 public key of the producer |
| `source.signature` | `[u8; 64]` | 64 B | Ed25519 signature over the fingerprint |
| `source.identity_type` | `enum` | 1 B | `Institutional` or `Pseudonymous` |
| `source.reputation_score` | `f32` | 4 B | Source reputation (0.0 – 1.0) |
| `temporal.timestamp_us` | `i64` | 8 B | Microseconds since Unix epoch |
| `temporal.source_clock` | `enum` | 1 B | `System`, `NTP`, or `GPS` |
| `temporal.precision_us` | `u32` | 4 B | Estimated clock precision |
| `custody_chain[]` | `Vec<CustodyEntry>` | ~105 B each | Signed custody records |
| `semantic_vector` | `Option<Vec<f16>>` | 0 or 768 B | 384-dimensional embedding (half-precision) |
| `integrity_hash` | `[u8; 32]` | 32 B | BLAKE3 hash of the entire dDNA (excluding this field) |

**CustodyEntry** (each ~105 bytes):

| Field | Type | Size |
|-------|------|------|
| `node_id` | `[u8; 32]` | 32 B |
| `timestamp_us` | `i64` | 8 B |
| `action` | `enum` | 1 B |
| `signature` | `[u8; 64]` | 64 B |

`CustodyAction` values: `Created`, `Received`, `Validated`, `Forwarded`.

### 3.2 Sizes

| Scenario | Approximate Size |
|----------|-----------------|
| Minimal (no vector, 1 custody entry) | ~350 bytes |
| With semantic vector | ~1,120 bytes |
| With 5 custody entries | ~1,640 bytes |

### 3.3 Serialization

- **Primary**: MessagePack via `rmp-serde` (compact, fast, binary)
- **API/Debug**: JSON via `serde_json`
- **Wire format**: length-prefixed MessagePack over TCP

### 3.4 Integrity Verification

The `integrity_hash` is computed as BLAKE3 over the concatenation of all fields in canonical order, *excluding* the integrity hash itself. This creates a self-verifying structure: modifying any field invalidates the hash.

```
integrity_hash = BLAKE3(
    version ||
    fingerprint.blake3 || fingerprint.sha3_512 || fingerprint.content_size ||
    source.public_key || source.signature || source.reputation_score ||
    temporal.timestamp_us || temporal.precision_us ||
    for each custody_entry: (node_id || timestamp_us || signature) ||
    if semantic_vector: (model_id || dimensions || values)
)
```

### 3.5 Signature Scheme

The producer signs the concatenation of both fingerprint hashes:

```
message = fingerprint.blake3 || fingerprint.sha3_512
signature = Ed25519_Sign(producer_secret_key, message)
```

Verification:

```
Ed25519_Verify(source.public_key, message, source.signature) → bool
```

Using dual hashes in the signed message provides defense in depth: if either hash algorithm is compromised in the future, the other still binds the signature to the content.

---

## 4. Adaptive Resonance Consensus (ARC)

ARC is a reputation-weighted Byzantine fault-tolerant consensus protocol that reaches finality in under 200ms without mining or staking.

### 4.1 Phases

```
  0ms          30ms         100ms        200ms
   ├─ PROPOSE ──┼─ VALIDATE ──┼─ RESONATE ──┤
   │            │             │             │
   │ Broadcast  │ Local       │ Aggregate   │
   │ to         │ checks in   │ weighted    │
   │ committee  │ parallel    │ votes       │
```

**Phase 1 — Propose (0–30ms):**
The proposer sends `(data, dDNA)` to all committee members via VTP.

**Phase 2 — Validate (30–100ms):**
Each committee member independently executes:

1. **Cryptographic verification**: Ed25519 signature valid? Integrity hash correct?
2. **Content verification**: BLAKE3 of content matches `fingerprint.blake3`?
3. **Temporal plausibility**: Timestamp not in the future (>60s tolerance)? Not too old (>24h)?
4. **Source reputation**: `reputation_score >= 0.3`?

Each check produces a pass/fail. The node casts a weighted vote:

- All checks pass → `Approve` (confidence = 1.0)
- ≥50% pass → `Abstain` (confidence = pass_ratio)
- <50% pass → `Reject` (confidence = 0.0)

**Phase 3 — Resonate (100–200ms):**
The proposer aggregates votes.

### 4.2 Scoring Formula

```
                    n
                   Σ  w(vote_i) × reputation_i
                   i=1
consensus_score = ─────────────────────────────
                    n
                   Σ  reputation_i
                   i=1
```

Where:

```
w(Approve)  = confidence_i
w(Reject)   = 0.0
w(Abstain)  = confidence_i × 0.5
```

### 4.3 Outcome

| Score Range | Outcome | Action |
|-------------|---------|--------|
| `≥ 0.67` | **Confirmed** | Data accepted, replicated to peers, score boosted |
| `0.33 – 0.67` | **Uncertain** | Escalate to larger committee |
| `< 0.33` | **Rejected** | Data isolated, source reputation penalized |

### 4.4 Reputation Update

After each validation round:

```
If correct:   reputation += (1.0 - reputation) × 0.05    // slow growth
If incorrect: reputation -= reputation × 0.15             // fast penalty
reputation = clamp(reputation, 0.0, 1.0)
```

Temporal decay: `reputation *= (1.0 - decay_rate)` per period, where `decay_rate = 0.01`.

### 4.5 Committee Selection

Default committee size: 7 (21 for critical domains).

Selection criteria (weighted):
- Domain competence: 40%
- Node reputation: 35%
- Geographic diversity: 25%

Minimum reputation for committee membership: 0.5.

---

## 5. VARCAVIA Transport Protocol (VTP)

### 5.1 Packet Structure

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID v4 | Unique packet identifier |
| `version` | `u8` | Protocol version |
| `priority` | `enum` | `Critical`, `High`, `Normal`, `Low`, `Background` |
| `source_node` | `[u8; 32]` | Sender node ID |
| `dest_node` | `Option<[u8; 32]>` | Recipient (None = broadcast) |
| `created_at_us` | `i64` | Creation timestamp |
| `ttl` | `u8` | Remaining hops (default: 16) |
| `is_delta` | `bool` | Payload is delta-compressed |
| `payload_hash` | `[u8; 32]` | BLAKE3 of payload (integrity check) |
| `payload` | `Vec<u8>` | Serialized data (possibly zstd-compressed) |

### 5.2 Semantic Priority

Priority is inferred from the data domain:

| Priority | Domains | Behavior |
|----------|---------|----------|
| `Critical` | emergency, disaster, critical_health | Transmitted immediately |
| `High` | health, finance, security | Priority queue |
| `Normal` | climate, science, education | Standard queue |
| `Low` | statistics, historical, archive | Best-effort |
| `Background` | backup, pre-positioning | Idle bandwidth only |

### 5.3 Compression

- **Full compression**: zstd level 3 (good speed/ratio tradeoff)
- **Delta compression**: XOR-based for same-length updates, full zstd fallback for different lengths

### 5.4 Wire Protocol

Messages are exchanged as length-prefixed JSON over TCP:

```
[4 bytes: message length as u32 big-endian][N bytes: JSON payload]
```

Message types: `Ping`, `Pong`, `VoteRequest`, `VoteResponse`, `DataAnnounce`, `DataRequest`, `DataResponse`, `StatusRequest`, `StatusResponse`.

### 5.5 CRDT Synchronization

- **LWW-Register** (Last-Writer-Wins): for mutable node state
- **G-Set** (Grow-only Set): for tracking seen dDNA hashes

Convergence is guaranteed: merging any two replicas always produces the same result, regardless of message ordering.

---

## 6. Query API

The Universal Access Gateway (UAG) exposes a REST API on port 8080.

### 6.1 Hero Endpoint — Verify a Fact

```bash
curl "http://localhost:8080/api/v1/verify?fact=Earth+diameter+is+12742+km"
```

Response:

```json
{
  "fact": "Earth diameter is 12742 km",
  "status": "verified",
  "data_dna": {
    "id": "bbf3d88a17a1864087e37cb8f73d419aa31ace8d7f1fd33b35572aeb519fd0c9",
    "fingerprint": {
      "blake3": "bbf3d88a17a1864087e37cb8f73d419aa31ace8d7f1fd33b35572aeb519fd0c9",
      "sha3_512": "631cc1a62afb821d3a701e7591d18ae81fa98a87e87972257a5b8575aaaa698b"
    },
    "source": {
      "public_key": "6829a2babf10dac17820626ba99c194ae03a06bae4711e4a14f0a9c68fa615b5",
      "identity_type": "Pseudonymous",
      "reputation": 0.5
    },
    "temporal": {
      "timestamp_us": 1774488423624402,
      "clock_source": "System",
      "precision_us": 1000
    },
    "version": 1
  },
  "score": 0.73,
  "duplicate": false,
  "warnings": []
}
```

Calling the same fact again returns `"status": "already_verified", "duplicate": true`.

### 6.2 Data Operations

```bash
# Insert
curl -X POST http://localhost:8080/api/v1/data \
  -H 'Content-Type: application/json' \
  -d '{"content":"Roma: 22°C","domain":"climate","source":"sensor-01"}'

# Get by ID
curl http://localhost:8080/api/v1/data/{id}

# Get Data DNA
curl http://localhost:8080/api/v1/data/{id}/dna

# Get score
curl http://localhost:8080/api/v1/data/{id}/score

# Query by domain
curl -X POST http://localhost:8080/api/v1/data/query \
  -H 'Content-Type: application/json' \
  -d '{"query":"","domain":"climate","limit":20}'

# Verify content integrity
curl -X POST http://localhost:8080/api/v1/data/verify \
  -H 'Content-Type: application/json' \
  -d '{"id":"...","content":"Roma: 22°C"}'

# Delete (soft)
curl -X DELETE http://localhost:8080/api/v1/data/{id}
```

### 6.3 Node & Network

```bash
# Node status
curl http://localhost:8080/api/v1/node/status

# Connected peers
curl http://localhost:8080/api/v1/node/peers

# Consensus state for a datum
curl http://localhost:8080/api/v1/node/consensus/{id}

# Network health
curl http://localhost:8080/api/v1/network/health

# Format translation (JSON → XML)
curl -X POST http://localhost:8080/api/v1/translate \
  -H 'Content-Type: application/json' \
  -d '{"data":{"city":"Roma","temp":22},"from_format":"json","to_format":"xml"}'
```

Supported format translations: `json↔csv`, `json↔xml`.

---

## 7. Clean Data Engine (CDE)

The CDE is a six-stage pipeline that processes every datum at ingestion.

### 7.1 Pipeline

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│ Stage 1  │───→│ Stage 2  │───→│ Stage 3  │───→│ Stage 4  │───→│ Stage 5  │───→│ Stage 6  │
│Hash Dedup│    │LSH Dedup │    │Semantic  │    │ Source   │    │Normalize │    │ Scoring  │
│          │    │          │    │Dedup     │    │Validate  │    │          │    │          │
│BLAKE3    │    │MinHash   │    │Trigram   │    │Ed25519   │    │VUF+zstd  │    │Composite │
│O(1)      │    │O(1) amort│    │Jaccard   │    │verify    │    │compress  │    │weighted  │
└──────────┘    └──────────┘    └──────────┘    └──────────┘    └──────────┘    └──────────┘
    │                │                │                │                              │
    │ REJECT if      │ WARN if        │ WARN if        │ REJECT if                   │
    │ exact match    │ sim > 0.85     │ sim > 0.90     │ sig invalid                 │
```

### 7.2 Stage Details

**Stage 1 — Exact Hash Dedup:**
Lookup `fingerprint.blake3` in a HashMap. O(1). If found: reject immediately.

**Stage 2 — LSH Near-Duplicate:**
Compute 128-function MinHash signature from 4-byte content shingles. Compare against all stored signatures. If Jaccard similarity > 0.85: emit warning (non-blocking).

**Stage 3 — Semantic Dedup:**
Extract character trigrams from UTF-8 text (lowercased, alphanumeric only). Compute Jaccard similarity against stored trigram sets. If similarity > 0.90: emit warning.

*Planned upgrade: replace trigrams with ONNX-based sentence embeddings (all-MiniLM-L6-v2, 384 dimensions) and HNSW k-NN index for true semantic similarity.*

**Stage 4 — Source Validation:**
Verify Ed25519 signature. Verify content matches fingerprint. Check source reputation ≥ 0.3. If signature invalid: reject.

**Stage 5 — Normalization:**
Detect content schema (JSON, CSV, plaintext, binary). Compress with zstd level 3. Store as VARCAVIA Universal Format (VUF).

**Stage 6 — Composite Scoring:**

```
score = 0.30 × source_reputation
      + 0.25 × coherence
      + 0.25 × freshness
      + 0.20 × validation_score

freshness = exp(-age_seconds / half_life)
validation_score = 1 - exp(-0.5 × num_validations)
coherence = 1.0 if no semantic warnings, 0.5 otherwise
```

---

## 8. Security Considerations

### 8.1 Cryptographic Choices

| Function | Algorithm | Rationale |
|----------|-----------|-----------|
| Content fingerprint (fast) | BLAKE3 | 6× faster than SHA-256, collision-resistant |
| Content fingerprint (strong) | SHA3-512 | Independent construction from BLAKE3 (defense in depth) |
| Digital signatures | Ed25519 | Fast verification, small keys (32 B), deterministic |
| Integrity hash | BLAKE3 | Self-verification of the entire dDNA structure |

### 8.2 Threat Model

| Threat | Mitigation |
|--------|-----------|
| Forged data | Ed25519 signature verification; content-fingerprint binding |
| Tampered data in transit | BLAKE3 payload hash in VTP packets |
| Replay attacks | Microsecond-precision timestamps; temporal plausibility checks |
| Sybil attacks | Reputation system with slow growth, fast penalty; committee diversity |
| Eclipse attacks | Geographic diversity in committee selection; multi-peer replication |
| Hash collision | Dual hashing (BLAKE3 + SHA3-512); both must match |
| Compromised node | Custody chain tracks every touch; reputation decay for bad behavior |

### 8.3 Limitations

- **No encryption at rest**: data is stored in plaintext in sled. Encryption at rest is planned.
- **No TLS**: P2P communication uses plaintext TCP. Migration to Noise protocol (via libp2p) is planned.
- **Trust-on-first-use**: new nodes start with reputation 0.5. A node that behaves correctly from the start is indistinguishable from a compromised one until it misbehaves.
- **Clock dependency**: temporal proofs depend on system clock accuracy (±60s tolerance).

---

## 9. Roadmap

| Phase | Status | Scope |
|-------|--------|-------|
| **Phase 1** — Foundations | Done | dDNA, storage (sled), CLI, all 6 crates compiling |
| **Phase 2** — Functional Node | Done | Live API, insert/query/verify, shared state |
| **Phase 3** — Multi-Node | Done | P2P TCP, ARC consensus, data replication, `run_local_network.sh` |
| **Phase 4** — CDE Complete | Done | Semantic dedup (trigrams), XML translator, dashboard, e2e tests |
| **Phase 5** — Publication | Done | Dockerfile, hero endpoint, landing page, README |
| **Phase 6** — AI Agents | Planned | ONNX embeddings, Python agents (dedup, classify, anomaly, coherence) |
| **Phase 7** — Production | Planned | libp2p migration, TLS, encryption at rest, GraphQL, benchmarks |
| **Phase 8** — Scale | Planned | Multi-machine clusters, NAT traversal, mobile nodes, LoRa/BLE transport |

---

## Appendix A: Quick Start

```bash
cargo build --bin varcavia-node
cargo run --bin varcavia-node -- --port 8080
curl "http://localhost:8080/api/v1/verify?fact=Water+boils+at+100+degrees+celsius"
```

## Appendix B: Multi-Node Local Network

```bash
bash scripts/run_local_network.sh 3
# Node 1: API :8080, P2P :8180
# Node 2: API :8081, P2P :8181
# Node 3: API :8082, P2P :8182
```

## Appendix C: Docker

```bash
docker build -t varcavia .
docker run -p 8080:8080 varcavia
```

---

*VARCAVIA — Because clean data is a right, not a privilege.*
