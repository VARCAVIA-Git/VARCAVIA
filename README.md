# 🌐 VARCAVIA

**Verifiable Autonomous Registry for Clean, Accessible, Validated & Interlinked Archives**

> Sistema Planetario di Dati Puliti — Un'infrastruttura decentralizzata dove ogni dato è pulito per definizione.

---

## Il Problema

Il pianeta genera 400+ exabyte di dati al giorno. L'80%+ è duplicato, corrotto, obsoleto o non verificabile. Questo costa 3+ trilioni di dollari/anno in pulizia, riconciliazione e decisioni errate.

## La Soluzione

VARCAVIA è un'infrastruttura peer-to-peer in cui ogni dato che entra viene automaticamente:

- **Verificato** nella fonte (firma crittografica Ed25519)
- **Certificato** con identità immutabile (Data DNA)
- **Deduplicato** in tempo reale (hash + LSH + embedding semantici)
- **Classificato** per dominio e affidabilità (AI distribuita)
- **Sincronizzato** globalmente in < 200ms (protocollo ARC)

## Quick Start

```bash
# 1. Clona il repository
git clone https://github.com/varcavia/varcavia.git
cd varcavia

# 2. Setup ambiente (installa Rust, Python, dipendenze)
bash scripts/setup.sh

# 3. Build
just build

# 4. Inizializza un nodo
just init

# 5. Avvia in modalità sviluppo
just dev

# 6. Avvia rete locale di 3 nodi
just network
```

## Architettura

```
┌─────────────────────────────────────────────────────────────┐
│              VARCAVIA — Stack a 7 Livelli                   │
├─────────────────────────────────────────────────────────────┤
│ L7  Universal Access Gateway (REST/GraphQL/gRPC)            │
│ L6  Predictive Mesh Intelligence (pre-posizionamento AI)    │
│ L5  Clean Data Engine (pulizia automatica 6 stadi)          │
│ L4  Adaptive Resonance Consensus (ARC, <200ms)              │
│ L3  Data DNA Layer (identità crittografica)                 │
│ L2  VARCAVIA Transport Protocol (VTP)                       │
│ L1  Physical Mesh Layer (TCP/BLE/LoRa/Satellite)            │
└─────────────────────────────────────────────────────────────┘
```

## Struttura Progetto

| Directory | Contenuto |
|-----------|-----------|
| `crates/ddna` | Data DNA — identità crittografica dei dati |
| `crates/vtp` | VARCAVIA Transport Protocol |
| `crates/arc` | Adaptive Resonance Consensus |
| `crates/cde` | Clean Data Engine (pipeline 6 stadi) |
| `crates/uag` | Universal Access Gateway (API server) |
| `crates/node` | Binary principale del nodo |
| `python/agents` | Micro-agenti AI (dedup, classificazione, anomalie) |
| `python/cde` | Componenti AI del Clean Data Engine |
| `configs/` | File di configurazione |
| `proto/` | Definizioni Protobuf |

## Stack Tecnologico

- **Rust** — core protocolli, crittografia, networking, API server
- **Python** — micro-agenti AI, embedding semantici, classificazione
- **RocksDB** — storage locale embedded
- **libp2p** — networking peer-to-peer
- **ONNX Runtime** — inference AI su CPU
- **Axum** — HTTP server async

## Comandi

```bash
just build          # Compila tutto
just test           # Esegui test Rust + Python
just dev            # Avvia nodo di sviluppo
just network 3      # Avvia rete locale (3 nodi)
just lint           # Lint Rust + Python
just fmt            # Formatta tutto
just doc            # Genera documentazione
just test-insert    # Inserisci dato di test via API
just test-status    # Query stato nodo
```

## Requisiti Hardware Minimi

- **OS**: Ubuntu 22.04+ (o qualsiasi Linux con glibc 2.35+)
- **RAM**: 8 GB (consigliati 16 GB)
- **Storage**: 10 GB liberi
- **CPU**: Qualsiasi x86_64 con 4+ core
- **GPU**: Non richiesta
- **Rete**: Localhost per sviluppo, qualsiasi connessione per produzione

## Innovazioni Chiave

1. **Data DNA (dDNA)** — Identità crittografica multi-livello per ogni dato
2. **Adaptive Resonance Consensus (ARC)** — Consenso in <200ms senza mining
3. **Gradient Flow Routing (GFR)** — Routing ispirato alla dinamica dei fluidi
4. **Clean Data Engine (CDE)** — Pipeline di purificazione a 6 stadi automatica
5. **Cross-Domain Coherence Check (CDCC)** — Anti-disinformazione cross-dominio
6. **Predictive Mesh Intelligence (PMI)** — Pre-posizionamento predittivo dei dati

## Licenza

AGPL-3.0 — Il codice è libero, le modifiche devono restare open-source.

## Stato

🚧 **In sviluppo attivo — Fase 1 (Proof of Concept)**

---

*VARCAVIA — Perché i dati puliti sono un diritto, non un privilegio.*
