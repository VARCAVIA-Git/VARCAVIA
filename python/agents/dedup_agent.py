"""
Dedup Agent — Agente per deduplicazione semantica continua.

Usa embedding vettoriali per trovare dati semanticamente equivalenti
ma strutturalmente diversi (es. stessa info in lingue diverse).
"""

from typing import Any

import numpy as np

from agents.base_agent import BaseAgent
from cde.embeddings import get_model


class DedupAgent(BaseAgent):
    """Agente di deduplicazione semantica."""

    def __init__(self, **kwargs: Any):
        super().__init__(agent_name="dedup", **kwargs)
        self.model = get_model()
        self.similarity_threshold = 0.9  # Soglia alta per dedup semantica

    async def process(self, request: dict[str, Any]) -> dict[str, Any]:
        """
        Controlla se un dato è un duplicato semantico di dati esistenti.

        Params nel request:
            text: il contenuto testuale del dato
            existing_vectors: lista di vettori di dati esistenti nello stesso dominio

        Returns:
            is_duplicate: bool
            most_similar_id: ID del dato più simile (se duplicato)
            similarity: score di similarità
        """
        text = request.get("text", "")
        existing = request.get("existing_vectors", [])

        if not text or not existing:
            return {"is_duplicate": False, "similarity": 0.0}

        # Calcola embedding del nuovo testo
        new_vec = self.model.encode([text])[0]

        # Confronta con esistenti
        max_sim = 0.0
        most_similar_id = None

        for item in existing:
            vec = np.array(item["vector"], dtype=np.float32)
            sim = self.model.cosine_similarity(new_vec, vec)
            if sim > max_sim:
                max_sim = sim
                most_similar_id = item.get("id")

        return {
            "is_duplicate": max_sim >= self.similarity_threshold,
            "most_similar_id": most_similar_id,
            "similarity": round(max_sim, 4),
            "embedding": new_vec.tolist(),
        }
