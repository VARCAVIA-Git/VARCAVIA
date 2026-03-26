# FAQ — Frequently Asked Questions

## General

### 1. What is VARCAVIA?

A decentralized infrastructure for verifying, certifying, and deduplicating data. Every datum gets a "Data DNA" — a cryptographic identity that proves who created it, what it contains, when it was created, and how reliable it is.

### 2. How is this different from blockchain?

| | VARCAVIA | Blockchain |
|---|---|---|
| Consensus speed | <200ms | Seconds to minutes |
| Energy use | Zero (reputation-based) | High (PoW) or moderate (PoS) |
| Storage | Each node stores what's relevant | Every node stores everything |
| Purpose | Data verification | Transaction ordering |
| Cost per operation | Free | Gas fees |

VARCAVIA doesn't maintain a global ledger. It maintains cryptographic proofs about individual data items with distributed consensus.

### 3. Who verifies the verifiers?

Reputation. Every node builds a reputation score based on its validation accuracy. Good behavior slowly builds reputation (5% growth per correct validation). Bad behavior is penalized harshly (15% penalty per incorrect validation). Reputation also decays over time (1% per period) — you can't coast on past performance.

New nodes start at 0.5 (neutral). A node must maintain >0.5 reputation to participate in validation committees.

### 4. What does the "score" mean?

The reliability score (0.0–1.0) is a composite of four factors:
- **Source reputation** (30%): How trustworthy is the data producer?
- **Coherence** (25%): Does this data conflict with semantically similar data?
- **Freshness** (25%): How recently was the data created?
- **Validation count** (20%): How many independent nodes have confirmed it?

A score of 0.73 means the system has moderate confidence in the data's reliability based on available evidence.

### 5. Does VARCAVIA determine if a fact is true?

No. VARCAVIA verifies the *integrity and provenance* of data, not its truthfulness. It can tell you:
- This data hasn't been tampered with
- It was created by key X at time T
- N independent nodes validated the cryptographic proofs
- It's not a duplicate of existing data

Whether "Earth's diameter is 12742 km" is actually true requires domain knowledge. VARCAVIA provides the infrastructure for building trust systems on top.

## Technical

### 6. Why dual hashing (BLAKE3 + SHA3)?

Defense in depth. BLAKE3 and SHA3 use completely different constructions (Merkle tree vs. Keccak sponge). If one algorithm is compromised in the future, the other still binds the signature to the content. The signature covers both hashes.

### 7. Why Ed25519 instead of ECDSA or RSA?

- Deterministic signatures (no nonce issues)
- Small keys (32 bytes) and signatures (64 bytes)
- Fast verification (~15,000 verifications/second on commodity CPU)
- No known practical attacks

### 8. Can it run on a Raspberry Pi?

Yes. The full node binary is ~15MB and runs with <100MB RAM. Tested on 8GB laptop. A Raspberry Pi 4 would work fine for single-node operation.

### 9. What happens if a node goes offline?

Nothing breaks. VARCAVIA is offline-first. The node's data persists in sled (embedded database). When it reconnects, CRDT synchronization (LWW-Register, G-Set) ensures eventual consistency without conflicts.

### 10. How does deduplication work?

Three layers:
1. **Exact** (Stage 1): BLAKE3 hash lookup. O(1). Catches identical data.
2. **Near-duplicate** (Stage 2): MinHash LSH with 128 hash functions. Catches data with >85% Jaccard similarity.
3. **Semantic** (Stage 3): Character trigram Jaccard. Catches rephrased content with >90% similarity. (Will be upgraded to ONNX neural embeddings.)

### 11. What's the throughput?

Single node on a laptop:
- Insert + CDE pipeline: ~5ms per datum (without AI embedding)
- Verification (dDNA creation + fingerprint): <1ms
- ARC consensus (3 local nodes): ~50ms
- API throughput: 1000+ req/s

### 12. Is the data encrypted?

Not currently. Data is stored in plaintext in sled. Encryption at rest and TLS for P2P are on the roadmap (Phase 7).

## Architecture

### 13. Why Rust?

Memory safety without garbage collection. Zero-cost abstractions. The entire node compiles to a single static binary. No runtime dependencies.

### 14. Why sled instead of RocksDB?

sled is a pure Rust embedded database. It compiles everywhere without C dependencies. RocksDB requires `librocksdb-dev` and adds 20+ seconds to build time.

### 15. Why not use libp2p from the start?

libp2p is the right long-term choice, but it adds significant compilation time (~2 minutes) and complexity. For the proof of concept, plain TCP with length-prefixed JSON is simpler to debug and faster to iterate on. Migration to libp2p is planned for Phase 7.

### 16. How does the 6-stage pipeline affect latency?

Stages 1-2 (hash + LSH dedup) are O(1) amortized. Stage 3 (semantic) is O(n) with small n (scales with stored data). Stage 4 (signature verification) is ~0.07ms. Stage 5 (normalization + zstd) is ~0.5ms. Stage 6 (scoring) is arithmetic. Total: <5ms for 10,000 stored items.

## Deployment

### 17. How do I run in production?

```bash
docker build -t varcavia .
docker run -p 8080:8080 -v varcavia-data:/data varcavia
```

For multi-node: pass `--peers` with addresses of other nodes.

### 18. Is there a hosted version?

Not yet. VARCAVIA is designed to be self-hosted. A public demo instance may be available for testing — check the GitHub repo.

### 19. What's the license?

AGPL-3.0. You can use it freely. If you modify it and offer it as a service, you must release your modifications.

### 20. How can I contribute?

See [CONTRIBUTING.md](../CONTRIBUTING.md). We welcome contributions in Rust code, documentation, tests, and protocol design.
