# VARCAVIA — Istruzioni per lo Sviluppo

## Che cos'è VARCAVIA

**Verifiable Autonomous Registry for Clean, Accessible, Validated & Interlinked Archives**

VARCAVIA è un'infrastruttura decentralizzata per dati puliti. Ogni dato che entra nel sistema viene automaticamente verificato, certificato, deduplicato e classificato. Il sistema è composto da 6 crate Rust + moduli Python per l'AI.

## Architettura Target (Hardware: Acer Aspire 5 / Ubuntu)

Tutto deve girare su un singolo laptop con 8-16 GB RAM. Nessun cluster, nessun cloud. Lo sviluppo simula una rete multi-nodo con processi locali.

**Vincoli hardware da rispettare SEMPRE:**
- RAM totale disponibile: max 12 GB per VARCAVIA
- Nessuna GPU richiesta (AI inference su CPU con ONNX)
- Storage: tutto in ~/varcavia-data/ (max 50 GB)
- Rete: simulata localmente via localhost multi-porta
- Nessuna dipendenza da servizi cloud esterni

## Stack Tecnologico

| Componente | Tecnologia | Motivo |
|---|---|---|
| Core protocolli | Rust 1.78+ | Performance, memory safety, zero-cost abstractions |
| AI/ML agents | Python 3.11+ | Ecosistema ML, rapidità di prototipazione |
| Database locale | RocksDB (via rust-rocksdb) | KV store embedded ad alte prestazioni |
| Networking P2P | libp2p (rust-libp2p) | Standard de-facto per reti decentralizzate |
| Serializzazione | MessagePack + Protobuf | Compatto e veloce |
| Crypto | ed25519-dalek, blake3, sha3 | Firme digitali + hashing |
| AI Inference | ONNX Runtime (Python) | Inference CPU ottimizzata |
| API Gateway | Axum (Rust) | HTTP server async velocissimo |
| Web Dashboard | React + Vite | Monitoring e admin locale |
| Testing | cargo test + pytest + just | Task runner unificato |
| Build System | Cargo workspace + just | Monorepo Rust |
| Containerizzazione | Podman (opzionale) | Alternativa leggera a Docker |

## Struttura del Monorepo

```
varcavia/
├── CLAUDE.md                    # QUESTO FILE - istruzioni per Claude Code
├── Cargo.toml                   # Workspace root
├── Justfile                     # Task runner (equivalente Makefile moderno)
├── .env                         # Configurazione locale
├── README.md                    # Documentazione pubblica
│
├── crates/                      # Codice Rust (6 crate)
│   ├── ddna/                    # Data DNA - identità crittografica dei dati
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs           # Entry point + re-exports
│   │       ├── fingerprint.rs   # Content Fingerprint (BLAKE3 + SHA3-512)
│   │       ├── identity.rs      # Source Identity (Ed25519)
│   │       ├── temporal.rs      # Temporal Proof (timestamp certificato)
│   │       ├── custody.rs       # Chain of Custody
│   │       ├── semantic.rs      # Semantic Vector (interfaccia per embedding)
│   │       ├── integrity.rs     # Integrity Proof
│   │       └── codec.rs         # Serializzazione/deserializzazione dDNA
│   │
│   ├── vtp/                     # VARCAVIA Transport Protocol
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── packet.rs        # Struttura pacchetti VTP
│   │       ├── priority.rs      # Semantic Priority Queuing
│   │       ├── routing.rs       # Gradient Flow Routing (GFR)
│   │       ├── compression.rs   # Delta Compression
│   │       ├── channel.rs       # Channel abstraction (UCA)
│   │       └── sync.rs          # Store-and-Forward + CRDT sync
│   │
│   ├── arc/                     # Adaptive Resonance Consensus
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── committee.rs     # Selezione Comitato di Risonanza
│   │       ├── validation.rs    # Validazione locale
│   │       ├── resonance.rs     # Propagazione a onda + BFT
│   │       ├── reputation.rs    # Sistema di reputazione nodi
│   │       └── scoring.rs       # Punteggio di affidabilità composito
│   │
│   ├── cde/                     # Clean Data Engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pipeline.rs      # Pipeline a 6 stadi orchestrator
│   │       ├── dedup.rs         # Deduplicazione (hash + LSH + semantica)
│   │       ├── validation.rs    # Validazione fonte + coerenza
│   │       ├── freshness.rs     # Controllo freschezza temporale
│   │       ├── normalize.rs     # Normalizzazione in VUF
│   │       └── scoring.rs       # Punteggio affidabilità composito
│   │
│   ├── uag/                     # Universal Access Gateway (API server)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs        # Axum HTTP server
│   │       ├── rest.rs          # REST API endpoints
│   │       ├── graphql.rs       # GraphQL schema + resolver
│   │       ├── translator.rs    # Universal Format Translator
│   │       └── middleware.rs    # Auth, rate limiting, logging
│   │
│   └── node/                    # Nodo VARCAVIA (binary principale)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs          # Entry point del nodo
│           ├── config.rs        # Configurazione nodo
│           ├── network.rs       # Gestione connessioni P2P
│           ├── storage.rs       # RocksDB wrapper
│           └── cli.rs           # CLI commands
│
├── python/                      # Moduli Python (AI/ML)
│   ├── pyproject.toml
│   ├── requirements.txt
│   ├── agents/                  # Micro-agenti AI
│   │   ├── __init__.py
│   │   ├── base_agent.py        # Classe base per tutti gli agenti
│   │   ├── dedup_agent.py       # Agente deduplicazione semantica
│   │   ├── classifier_agent.py  # Agente classificazione dominio
│   │   ├── anomaly_agent.py     # Agente rilevamento anomalie
│   │   └── coherence_agent.py   # Agente coerenza cross-domain (CDCC)
│   ├── cde/                     # Clean Data Engine (componenti AI)
│   │   ├── __init__.py
│   │   ├── embeddings.py        # Calcolo embedding semantici
│   │   ├── lsh.py               # Locality-Sensitive Hashing
│   │   └── maturation.py        # Data Maturation Protocol
│   ├── utils/                   # Utilità condivise
│   │   ├── __init__.py
│   │   └── onnx_runtime.py      # Wrapper ONNX Runtime per CPU
│   └── tests/
│       ├── test_agents.py
│       └── test_embeddings.py
│
├── proto/                       # Protobuf definitions
│   ├── ddna.proto               # Schema Data DNA
│   ├── vtp.proto                # Schema pacchetti VTP
│   ├── arc.proto                # Schema messaggi ARC
│   └── node.proto               # Schema comunicazione nodi
│
├── configs/                     # File di configurazione
│   ├── node_default.toml        # Config nodo di default
│   ├── network_local.toml       # Config rete locale (dev)
│   └── domains.toml             # Domini di dati supportati
│
├── scripts/                     # Script di utilità
│   ├── setup.sh                 # Setup iniziale ambiente
│   ├── run_local_network.sh     # Avvia rete locale multi-nodo
│   ├── seed_test_data.py        # Popola con dati di test
│   └── benchmark.sh             # Benchmark prestazioni
│
├── web/                         # Dashboard web
│   └── dashboard/
│       ├── package.json
│       ├── vite.config.ts
│       └── src/
│           ├── App.tsx
│           └── main.tsx
│
├── tests/                       # Test di integrazione
│   ├── integration/
│   │   ├── test_ddna_flow.rs
│   │   ├── test_arc_consensus.rs
│   │   └── test_cde_pipeline.rs
│   └── e2e/
│       └── test_full_node.rs
│
└── docs/                        # Documentazione
    ├── ARCHITECTURE.md          # Architettura dettagliata
    ├── PROTOCOLS.md             # Specifiche protocolli
    ├── API.md                   # Documentazione API
    └── DEVELOPMENT.md           # Guida sviluppatore
```

## Ordine di Sviluppo (CRITICO — seguire questa sequenza)

### FASE 1: Fondamenta (Settimane 1-4)
**Obiettivo: dDNA + Storage + CLI base**

1. **Setup workspace** — Cargo.toml workspace, Justfile, .env, dipendenze
2. **crate `ddna`** — Implementare TUTTO il Data DNA:
   - `fingerprint.rs`: doppio hash BLAKE3 + SHA3-512
   - `identity.rs`: generazione keypair Ed25519, firma e verifica
   - `temporal.rs`: timestamp con precisione al microsecondo
   - `custody.rs`: catena di custodia come lista di firme
   - `semantic.rs`: struct per embedding vector (la AI lo calcola, Rust lo trasporta)
   - `integrity.rs`: hash dell'intero dDNA per auto-verifica
   - `codec.rs`: serializzazione MessagePack
   - Test unitari per ogni modulo
3. **crate `node`** — Scheletro base:
   - `config.rs`: lettura config TOML
   - `storage.rs`: wrapper RocksDB (put/get/delete/scan)
   - `cli.rs`: comandi base (init, status, insert, query)
   - `main.rs`: bootstrap nodo singolo

### FASE 2: Rete e Trasporto (Settimane 5-8)
**Obiettivo: nodi che comunicano tra loro**

4. **crate `vtp`** — Protocollo di trasporto:
   - `packet.rs`: struttura pacchetti con header semantico
   - `priority.rs`: coda a priorità basata su dominio del dato
   - `compression.rs`: delta compression con zstd
   - `channel.rs`: astrazione canale su TCP localhost
   - `sync.rs`: CRDT base per sincronizzazione stato
5. **Networking in `node`**:
   - `network.rs`: discovery nodi via mDNS locale + libp2p
   - Scambio dDNA tra nodi
   - Script `run_local_network.sh`: avvia 3-5 nodi su porte diverse

### FASE 3: Consenso (Settimane 9-12)
**Obiettivo: validazione distribuita funzionante**

6. **crate `arc`** — Adaptive Resonance Consensus:
   - `committee.rs`: selezione comitato (per ora round-robin, poi per reputazione)
   - `validation.rs`: verifica dDNA + controlli base
   - `resonance.rs`: propagazione voti + soglia 2/3
   - `reputation.rs`: punteggio nodo (inizia semplice, poi raffina)
   - `scoring.rs`: calcolo affidabilità dato post-consenso
7. **Integrazione ARC nel nodo**: quando un dato viene inserito, il nodo avvia il consenso

### FASE 4: Pulizia Dati (Settimane 13-16)
**Obiettivo: Clean Data Engine funzionante**

8. **crate `cde`** — Pipeline di pulizia:
   - `dedup.rs`: deduplicazione hash esatto + LSH
   - `validation.rs`: verifica firma fonte
   - `freshness.rs`: confronto timestamp con versioni esistenti
   - `normalize.rs`: conversione in formato VUF interno
   - `scoring.rs`: punteggio composito
   - `pipeline.rs`: orchestrazione dei 6 stadi
9. **Python `cde/`** — Componenti AI:
   - `embeddings.py`: calcolo embedding con modello ONNX (all-MiniLM-L6-v2)
   - `lsh.py`: Locality-Sensitive Hashing per near-duplicates
   - Setup comunicazione Rust↔Python via socket Unix o subprocess+JSON

### FASE 5: AI Agents (Settimane 17-20)
**Obiettivo: micro-agenti intelligenti**

10. **Python `agents/`**:
    - `base_agent.py`: loop base, comunicazione col nodo Rust
    - `dedup_agent.py`: deduplicazione semantica continua
    - `classifier_agent.py`: classificazione dati per dominio
    - `anomaly_agent.py`: rilevamento dati anomali
    - `coherence_agent.py`: CDCC cross-domain check
11. **Integrazione**: il nodo Rust lancia gli agenti Python come processi figli

### FASE 6: API Gateway + Dashboard (Settimane 21-24)
**Obiettivo: sistema usabile dall'esterno**

12. **crate `uag`** — API server:
    - `server.rs`: Axum server con routing
    - `rest.rs`: CRUD endpoints (insert/query/verify/status)
    - `graphql.rs`: schema GraphQL per query complesse
    - `translator.rs`: conversione JSON↔VUF↔CSV base
    - `middleware.rs`: logging, CORS, rate limiting
13. **Web Dashboard**: React app per monitorare lo stato dei nodi, dati inseriti, punteggi

## Convenzioni di Codice

### Rust
- Edition 2021, MSRV 1.78
- `#![deny(clippy::all)]` in ogni crate
- Errori: usa `thiserror` per librerie, `anyhow` per binari
- Async: `tokio` runtime multi-thread
- Serializzazione: `serde` + `rmp-serde` (MessagePack)
- Logging: `tracing` con `tracing-subscriber`
- Test: ogni modulo ha `#[cfg(test)] mod tests` inline + file test separati per integrazione
- Documentazione: `///` per ogni funzione pubblica

### Python
- Python 3.11+, type hints ovunque
- Formattatore: `ruff format`
- Linter: `ruff check`
- Test: `pytest` con coverage
- Ambiente: `venv` in `.venv/`
- Comunicazione con Rust: JSON via stdin/stdout o socket Unix

### Naming
- Rust: snake_case per funzioni/variabili, CamelCase per tipi
- Python: snake_case ovunque, CamelCase per classi
- Proto: CamelCase per messaggi, snake_case per campi
- Config TOML: snake_case

### Commit
- Prefissi: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`
- Un commit per feature logica
- Test devono passare prima di ogni commit

## Comandi Principali (Justfile)

```just
# Setup completo ambiente di sviluppo
setup:
    rustup update stable
    cargo install just
    pip install -r python/requirements.txt
    mkdir -p ~/varcavia-data

# Build tutto
build:
    cargo build --workspace

# Test tutto
test:
    cargo test --workspace
    cd python && pytest

# Avvia un singolo nodo in modalità sviluppo
dev:
    cargo run --bin varcavia-node -- --config configs/node_default.toml

# Avvia rete locale di 3 nodi
network:
    bash scripts/run_local_network.sh 3

# Benchmark
bench:
    cargo bench --workspace

# Lint tutto
lint:
    cargo clippy --workspace -- -D warnings
    cd python && ruff check .

# Formatta tutto
fmt:
    cargo fmt --all
    cd python && ruff format .
```

## Specifiche Tecniche Dettagliate

### Data DNA (dDNA) — Struttura Esatta

```rust
pub struct DataDna {
    /// Versione del protocollo dDNA
    pub version: u8,                    // Sempre 1 per ora
    
    /// Content Fingerprint: doppio hash del contenuto
    pub fingerprint: ContentFingerprint,
    
    /// Identità verificata del produttore
    pub source: SourceIdentity,
    
    /// Timestamp certificato di creazione
    pub temporal: TemporalProof,
    
    /// Catena di custodia (ogni nodo che ha toccato il dato)
    pub custody_chain: Vec<CustodyEntry>,
    
    /// Vettore semantico (768 dimensioni, f16)
    pub semantic_vector: Option<SemanticVector>,
    
    /// Hash dell'intero dDNA per auto-verifica
    pub integrity_hash: [u8; 32],
}

pub struct ContentFingerprint {
    pub blake3: [u8; 32],       // BLAKE3 hash (veloce)
    pub sha3_512: [u8; 64],     // SHA3-512 hash (sicuro)
    pub content_size: u64,       // Dimensione originale in bytes
}

pub struct SourceIdentity {
    pub public_key: [u8; 32],    // Ed25519 public key
    pub signature: [u8; 64],     // Firma del content fingerprint
    pub identity_type: IdentityType, // Institutional | Pseudonymous
    pub reputation_score: f32,   // 0.0 - 1.0
}

pub struct TemporalProof {
    pub timestamp_us: i64,       // Microsecondi da Unix epoch
    pub source_clock: ClockSource, // GPS | NTP | System
    pub precision_us: u32,       // Precisione stimata in microsecondi
}

pub struct CustodyEntry {
    pub node_id: [u8; 32],      // ID del nodo
    pub timestamp_us: i64,
    pub action: CustodyAction,   // Created | Received | Validated | Forwarded
    pub signature: [u8; 64],     // Firma del nodo
}

pub struct SemanticVector {
    pub model_id: String,        // es. "all-MiniLM-L6-v2"
    pub dimensions: u16,         // 384 per MiniLM
    pub values: Vec<f16>,        // Valori del vettore
}
```

### ARC Consensus — Algoritmo

```
PROCEDURA: ValidaDato(dato, dDNA)

1. SELEZIONE COMITATO:
   - Calcola dominio = ClassificaDominio(dDNA.semantic_vector)
   - Seleziona N nodi (N = 7 per dominio comune, 21 per critico) dove:
     * competenza[nodo][dominio] > soglia_minima
     * reputazione[nodo] > 0.5
     * diversità_geografica(comitato) > 3 regioni
   - Se non abbastanza nodi qualificati: espandi a nodi generalisti

2. FASE PROPOSTA (0-30ms locali):
   - Invia (dato, dDNA) a tutti i membri del comitato via VTP
   - Timeout: 100ms (se un membro non risponde, escludi)

3. FASE VALIDAZIONE (30-100ms):
   Ogni membro esegue indipendentemente:
   a. Verifica crittografica: firma Ed25519 valida? hash corretti?
   b. Verifica temporale: timestamp plausibile? (non futuro, non troppo vecchio)
   c. Verifica coerenza: dato coerente con dati correlati nel DB locale?
   d. Verifica duplicazione: dato già presente? (hash + LSH)
   e. Produce voto = { approve | reject | abstain } pesato per reputazione

4. FASE RISONANZA (100-200ms):
   - Raccogli voti pesati
   - Calcola score = Σ(voto_i * peso_reputazione_i) / Σ(peso_reputazione_i)
   - Se score ≥ 0.67: CONFERMATO → propaga onda di conferma
   - Se score < 0.33: RIFIUTATO → segnala e isola
   - Se 0.33 ≤ score < 0.67: INCERTO → escalation a comitato più grande

5. POST-CONSENSO:
   - Aggiorna reputazione dei validatori
   - Aggiorna punteggio del dato
   - Replica dato sui nodi in base alla domanda prevista
```

### Clean Data Engine — Pipeline

```
PIPELINE CDE (eseguita su ogni dato in ingresso):

STADIO 1 — DEDUP HASH ESATTO
  Input: dato + dDNA
  Operazione: cerca dDNA.fingerprint.blake3 nel DB locale
  Se trovato: STOP, restituisci riferimento al dato esistente
  Complessità: O(1) lookup in RocksDB

STADIO 2 — DEDUP NEAR-DUPLICATE (LSH)
  Input: dato + dDNA  
  Operazione: calcola MinHash del contenuto, cerca in tabelle LSH
  Se similarità > 0.85: marca come possibile duplicato, segnala
  Complessità: O(1) ammortizzato con tabelle hash pre-calcolate

STADIO 3 — DEDUP SEMANTICA (richiede Python agent)
  Input: dDNA.semantic_vector
  Operazione: ricerca k-NN nello spazio embedding (HNSW index)
  Se distanza coseno < 0.1 con dato esistente: possibile duplicato semantico
  Complessità: O(log N) con indice HNSW

STADIO 4 — VALIDAZIONE FONTE
  Input: dDNA.source
  Operazione: verifica firma Ed25519, controlla reputazione fonte
  Se firma invalida: RIFIUTA
  Se reputazione < 0.3: segnala come bassa affidabilità

STADIO 5 — NORMALIZZAZIONE
  Input: dato in formato originale
  Operazione: converte in VUF (VARCAVIA Universal Format)
  VUF = MessagePack({ schema: <tipo_schema>, payload: <dati_compressi_zstd> })

STADIO 6 — SCORING
  Input: risultati di tutti gli stadi precedenti
  Operazione: calcola punteggio composito
  score = 0.3 * rep_fonte + 0.25 * coerenza + 0.25 * freschezza + 0.2 * validazioni
  Output: dato + dDNA + score → storage
```

### Configurazione Nodo (node_default.toml)

```toml
[node]
name = "varcavia-dev-01"
data_dir = "~/varcavia-data/node-01"
log_level = "info"

[network]
listen_addr = "127.0.0.1"
listen_port = 7700
bootstrap_nodes = []           # Vuoto per primo nodo
max_peers = 50
mdns_enabled = true            # Discovery locale automatico

[storage]
engine = "rocksdb"
max_size_gb = 10
compression = "zstd"
cache_size_mb = 256

[arc]
committee_size = 7
confirmation_threshold = 0.67
validation_timeout_ms = 500    # Più alto per dev locale
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
bind_addr = "127.0.0.1:8080"
cors_origins = ["http://localhost:5173"]
rate_limit_per_sec = 100
```

### API REST Endpoints

```
POST   /api/v1/data              — Inserisci un nuovo dato
GET    /api/v1/data/{id}         — Recupera un dato per ID (dDNA hash)
GET    /api/v1/data/{id}/dna     — Recupera solo il dDNA di un dato
POST   /api/v1/data/query        — Query semantica sui dati
POST   /api/v1/data/verify       — Verifica autenticità di un dato
GET    /api/v1/data/{id}/score   — Punteggio di affidabilità
DELETE /api/v1/data/{id}         — Marca dato come obsoleto (soft delete)

GET    /api/v1/node/status       — Stato del nodo
GET    /api/v1/node/peers        — Lista nodi connessi
GET    /api/v1/node/stats        — Statistiche (dati gestiti, uptime, ecc.)

GET    /api/v1/network/health    — Salute della rete
GET    /api/v1/network/topology  — Topologia rete (nodi + connessioni)

POST   /api/v1/translate         — Converti dato tra formati (JSON↔CSV↔XML)

Tutti gli endpoint restituiscono JSON.
Tutti gli endpoint di scrittura richiedono firma Ed25519 nell'header X-Varcavia-Signature.
```

### Comunicazione Rust ↔ Python

La comunicazione tra il nodo Rust e gli agenti Python avviene via **JSON-RPC su socket Unix**:

```
Socket path: /tmp/varcavia-agent-{node_id}.sock

Rust → Python (richieste):
{
  "jsonrpc": "2.0",
  "method": "compute_embedding",
  "params": { "text": "contenuto del dato", "model": "all-MiniLM-L6-v2" },
  "id": 1
}

Python → Rust (risposte):
{
  "jsonrpc": "2.0",
  "result": { "vector": [0.023, -0.145, ...], "dimensions": 384 },
  "id": 1
}

Metodi disponibili:
- compute_embedding(text, model) → vector
- check_semantic_similarity(vector_a, vector_b) → similarity_score
- classify_domain(text) → domain_label + confidence
- detect_anomaly(data_json, context) → anomaly_score
- check_coherence(data_json, related_data[]) → coherence_score
```

## Testing Strategy

### Unit Test (ogni modulo)
- Ogni file `.rs` ha `#[cfg(test)] mod tests` con almeno 3 test
- Ogni file `.py` ha test corrispondente in `python/tests/`
- Coverage target: 80%

### Integration Test (cross-crate)
- `tests/integration/test_ddna_flow.rs`: crea dato → genera dDNA → verifica → serializza → deserializza
- `tests/integration/test_arc_consensus.rs`: simula 5 nodi → inserisci dato → verifica consenso
- `tests/integration/test_cde_pipeline.rs`: dato sporco → pipeline CDE → verifica pulizia

### E2E Test
- `tests/e2e/test_full_node.rs`: avvia nodo → inserisci via API → query → verifica risultato

### Performance Benchmark
- dDNA creation: target < 1ms per dato
- ARC consensus (5 nodi locali): target < 50ms
- CDE pipeline (senza AI): target < 5ms per dato
- CDE pipeline (con AI embedding): target < 50ms per dato
- API throughput: target > 1000 req/s per nodo singolo

## Come Eseguire il Progetto (Comandi)

```bash
# 1. Setup iniziale (una volta sola)
bash scripts/setup.sh

# 2. Build
just build

# 3. Test
just test

# 4. Avvia un nodo singolo
just dev

# 5. Avvia rete locale di 3 nodi  
just network

# 6. Inserisci dato di test
curl -X POST http://localhost:8080/api/v1/data \
  -H "Content-Type: application/json" \
  -d '{"content": "La temperatura a Roma è 22°C", "domain": "climate", "source": "test"}'

# 7. Query
curl http://localhost:8080/api/v1/data/{id}

# 8. Dashboard web
cd web/dashboard && npm run dev
# → http://localhost:5173
```

## Note per Claude Code

- **NON installare Docker** — usa processi locali per simulare la rete
- **NON usare GPU** — tutto su CPU, modelli AI piccoli (<100MB)
- **NON connetterti a servizi esterni** — tutto locale, offline-first
- **USA `just`** come task runner — più pulito di make per questo progetto
- Quando crei un nuovo file, aggiungilo anche alla struttura nel `Cargo.toml` corretto
- Ogni crate deve compilare indipendentemente (`cargo build -p ddna`)
- I test devono passare ad ogni fase prima di procedere alla successiva
- Se un'operazione richiede più di 5 secondi su laptop, è troppo lenta — ottimizza
- Il modello ONNX va scaricato una volta e messo in `models/` — vedi setup.sh
