# Integrazione CDE con agenti AI Python

## Stadio 3 della pipeline CDE (deduplicazione semantica) è in TODO

La pipeline CDE attualmente salta lo Stadio 3 (dedup semantica) perché richiede:
1. Calcolo embedding tramite agente Python (all-MiniLM-L6-v2 via ONNX)
2. Indice HNSW per ricerca k-NN nello spazio embedding
3. Socket Unix JSON-RPC per comunicazione Rust ↔ Python

## Suggerimenti

- **HNSW index**: usare `hnsw_rs` (crate Rust puro) per evitare la dipendenza Python per la ricerca, mantenendo Python solo per il calcolo embedding
- **Batching**: accumulare dati in batch da 32 prima di invocare Python per l'embedding (molto più efficiente)
- **Cache embedding**: salvare gli embedding calcolati nello storage sled per evitare ricalcoli
- **Fallback**: se l'agente Python non è disponibile, proseguire con la pipeline saltando lo stadio 3 (come fa ora)
