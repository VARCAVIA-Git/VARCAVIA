# VARCAVIA — Architettura Dettagliata

## Visione d'Insieme

VARCAVIA è organizzata come una rete peer-to-peer stratificata con tre macro-componenti:

1. **Transport Fabric** — Comunicazione multi-mezzo auto-ottimizzante
2. **Truth Core** — Validazione, certificazione e consenso
3. **Autonomous Intelligence** — AI distribuita per pulizia e previsione

## Stack a 7 Livelli

### L1 — Physical Mesh Layer (Substrato Fisico)
Astrae il mezzo di trasporto con Unified Channel Abstraction (UCA).
Supporta: TCP, WebSocket, BLE, LoRaWAN, sincronizzazione offline.
Implementazione: `crates/vtp/src/channel.rs`

### L2 — VARCAVIA Transport Protocol (VTP)
Protocollo di trasporto nativo. Sostituisce TCP/IP per comunicazioni VARCAVIA.
- Semantic Priority Queuing (`priority.rs`)
- Gradient Flow Routing (`routing.rs`)
- Delta Compression con zstd (`compression.rs`)
- Store-and-Forward con CRDT (`sync.rs`)
Implementazione: `crates/vtp/`

### L3 — Data DNA Layer
Ogni dato riceve un pacchetto di metadati crittografici immutabili:
- Content Fingerprint (BLAKE3 + SHA3-512)
- Source Identity (Ed25519)
- Temporal Proof (timestamp al microsecondo)
- Chain of Custody
- Semantic Vector (embedding AI)
- Integrity Proof (hash auto-verifica)
Implementazione: `crates/ddna/`

### L4 — Adaptive Resonance Consensus (ARC)
Consenso distribuito in <200ms tramite:
1. Selezione Comitato di Risonanza (7-21 nodi per competenza)
2. Validazione locale parallela
3. Aggregazione voti con BFT e propagazione a onda
Implementazione: `crates/arc/`

### L5 — Clean Data Engine (CDE)
Pipeline automatica a 6 stadi:
1. Deduplicazione hash esatto (BLAKE3 lookup)
2. Deduplicazione near-duplicate (LSH/MinHash)
3. Deduplicazione semantica (embedding cosine similarity)
4. Validazione fonte (Ed25519 + reputazione)
5. Normalizzazione in VUF (MessagePack + zstd)
6. Scoring affidabilità composito
Implementazione: `crates/cde/` + `python/cde/`

### L6 — Predictive Mesh Intelligence (PMI)
AI distribuita per pre-posizionamento dati:
- Orizzonte breve (secondi): serie temporali traffico
- Orizzonte medio (ore): correlazione eventi
- Orizzonte lungo (settimane): trend stagionali
Implementazione: `python/agents/` (futuro)

### L7 — Universal Access Gateway (UAG)
API server che espone VARCAVIA al mondo:
- REST API (Axum)
- GraphQL
- Universal Format Translator (JSON↔CSV↔XML↔Protobuf)
Implementazione: `crates/uag/`

## Flusso di un Dato

```
Dato in ingresso
       │
       ▼
┌──────────────┐
│ L7: UAG      │ ← Riceve via REST/GraphQL
│  Traduzione  │ ← Converte formato in VUF
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ L3: dDNA     │ ← Genera Data DNA (fingerprint + firma + timestamp)
│  Creazione   │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ L5: CDE      │ ← Pipeline 6 stadi (dedup + validazione + scoring)
│  Pipeline    │ ← Chiama Python agents per embedding e classificazione
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ L4: ARC      │ ← Seleziona comitato → validazione → consenso
│  Consenso    │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ L2: VTP      │ ← Propaga a rete con priorità semantica
│  Trasporto   │ ← Delta compression + routing ottimale
└──────┬───────┘
       │
       ▼
  Dato certificato
  e distribuito
```

## Comunicazione Rust ↔ Python

I micro-agenti Python comunicano col nodo Rust via JSON-RPC su socket Unix:

```
┌─────────────┐    JSON-RPC     ┌─────────────────┐
│ Nodo Rust   │◄──────────────►│ Python Agents    │
│ (crates/)   │  Unix Socket   │ (python/agents/) │
└─────────────┘                └─────────────────┘
```

Il nodo Rust è il processo principale. Gli agenti Python sono processi figli
lanciati dal nodo e supervisionati. Se un agente crasha, viene riavviato.

## Storage

Ogni nodo usa RocksDB con queste column families:

| CF | Contenuto | Chiave | Valore |
|---|---|---|---|
| `data` | Dati raw | blake3 hash | contenuto compresso zstd |
| `ddna` | Data DNA | blake3 hash | dDNA serializzato MessagePack |
| `scores` | Punteggi | blake3 hash | ReliabilityScore |
| `custody` | Custodia | blake3 hash | Lista CustodyEntry |
| `peers` | Nodi noti | node_id | NodeInfo |
| `reputation` | Reputazioni | node_id | NodeReputation |
| `lsh` | Indice LSH | band_hash | Set di document IDs |
