# VARCAVIA — Specifiche Protocolli

## 1. Data DNA (dDNA) Protocol v1

### Struttura
| Campo | Tipo | Dimensione | Descrizione |
|-------|------|-----------|-------------|
| version | u8 | 1 byte | Sempre 1 |
| fingerprint.blake3 | [u8; 32] | 32 bytes | Hash veloce per lookup |
| fingerprint.sha3_512 | [u8; 64] | 64 bytes | Hash crittografico |
| fingerprint.content_size | u64 | 8 bytes | Dimensione originale |
| source.public_key | [u8; 32] | 32 bytes | Ed25519 pubkey |
| source.signature | [u8; 64] | 64 bytes | Firma del fingerprint |
| source.identity_type | enum | 1 byte | Institutional/Pseudonymous |
| source.reputation | f32 | 4 bytes | 0.0 - 1.0 |
| temporal.timestamp_us | i64 | 8 bytes | Microsecondi Unix epoch |
| temporal.source_clock | enum | 1 byte | System/NTP/GPS |
| temporal.precision_us | u32 | 4 bytes | Precisione stimata |
| custody_chain | Vec | variabile | Lista di CustodyEntry |
| semantic_vector | Option | variabile | 384 × f16 = 768 bytes |
| integrity_hash | [u8; 32] | 32 bytes | BLAKE3 dell'intero dDNA |

### Dimensione Tipica
- Senza semantic vector: ~350 bytes
- Con semantic vector: ~1100 bytes
- Con 5 entry di custodia: ~1600 bytes

### Serializzazione
Formato primario: MessagePack (compatto, veloce)
Formato alternativo: JSON (debug e API)

## 2. ARC Protocol v1

### Fasi
1. **Proposta** (0-30ms): broadcast a Comitato di Risonanza
2. **Validazione** (30-100ms): controlli locali paralleli
3. **Risonanza** (100-200ms): aggregazione BFT + propagazione

### Parametri Default
| Parametro | Valore | Descrizione |
|-----------|--------|-------------|
| committee_size | 7 | Nodi per comitato (dominio comune) |
| committee_size_critical | 21 | Nodi per dominio critico |
| confirmation_threshold | 0.67 | Soglia conferma (2/3) |
| rejection_threshold | 0.33 | Soglia rifiuto |
| validation_timeout_ms | 200 | Timeout validazione |
| max_committee_latency_ms | 50 | Latenza max tra membri |

### Selezione Comitato
Criteri pesati: competenza dominio (40%), reputazione (35%), diversità geografica (25%)

## 3. VTP Protocol v1

### Header Pacchetto
| Campo | Tipo | Descrizione |
|-------|------|-------------|
| id | UUID | Identificativo univoco |
| version | u8 | Sempre 1 |
| priority | enum | Critical/High/Normal/Low/Background |
| source_node | [u8; 32] | ID nodo sorgente |
| dest_node | Option<[u8; 32]> | Destinazione (None = broadcast) |
| created_at_us | i64 | Timestamp creazione |
| ttl | u8 | Hop rimanenti (default 16) |
| is_delta | bool | Payload è delta-compresso |
| payload_hash | [u8; 32] | BLAKE3 del payload |
| payload | Vec<u8> | Dati (eventualmente compressi) |

### Routing (Gradient Flow)
Potenziale di un link: `P = latency × (1 + load) / (reliability × bandwidth)`
Aggiornamento: ogni 50ms
Algoritmo: segui gradiente decrescente verso destinazione

### Compressione
Algoritmo: zstd livello 3 (compromesso velocità/ratio)
Delta: XOR-based per dati stessa lunghezza, fallback a zstd completo

## 4. JSON-RPC Agent Protocol

### Socket
Path: `/tmp/varcavia-agent-{node_id}.sock`
Formato: JSON-RPC 2.0

### Metodi
| Metodo | Direzione | Descrizione |
|--------|-----------|-------------|
| agent.poll | Rust→Python | Richiedi lavoro per un agente |
| agent.result | Python→Rust | Restituisci risultato |
| compute_embedding | Rust→Python | Calcola embedding testo |
| check_similarity | Rust→Python | Confronta due vettori |
| classify_domain | Rust→Python | Classifica per dominio |
| detect_anomaly | Rust→Python | Rilevamento anomalie |
| check_coherence | Rust→Python | CDCC cross-domain |
