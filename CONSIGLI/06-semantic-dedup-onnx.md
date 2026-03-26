# Upgrade deduplicazione semantica: da trigram a ONNX embedding

## Stato attuale
Lo Stadio 3 della pipeline CDE usa character trigram Jaccard similarity.
Funziona bene per testi quasi identici ma non cattura la similarità semantica reale.

Esempio che NON viene catturato:
- "La temperatura a Roma è 22 gradi" vs "Il termometro nella capitale segna 22°C"
- Stessi fatti, parole diverse → trigram similarity bassa

## Piano di upgrade (Fase 5)
1. Scaricare all-MiniLM-L6-v2 ONNX (~22MB) in `models/`
2. Implementare `python/cde/embeddings.py` con ONNX Runtime
3. Comunicazione Rust↔Python via JSON-RPC su socket Unix
4. Sostituire `SemanticDedupIndex.check_semantic_duplicate()` con embedding + cosine distance
5. Usare indice HNSW (`hnsw_rs` crate) per ricerca k-NN efficiente

## Approccio ibrido consigliato
Mantenere il trigram check come fallback veloce (O(n) con n piccolo),
e usare embedding solo per i dati che superano lo stadio LSH.
Questo riduce le chiamate a Python del ~90%.
