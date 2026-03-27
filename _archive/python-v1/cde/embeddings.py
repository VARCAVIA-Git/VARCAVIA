"""
Embedding Calculator — Calcolo embedding semantici con ONNX Runtime.

Modello default: all-MiniLM-L6-v2 (384 dimensioni, ~80MB, CPU-only).
Questo modulo è il cuore della deduplicazione semantica e della classificazione.
"""

import logging
import os
from pathlib import Path
from typing import Optional

import numpy as np

logger = logging.getLogger("varcavia.embeddings")

# Lazy import per ONNX Runtime
_ort = None
_tokenizer = None


def _load_onnx():
    """Carica ONNX Runtime in modo lazy."""
    global _ort
    if _ort is None:
        import onnxruntime as ort
        _ort = ort
    return _ort


class EmbeddingModel:
    """Wrapper per il modello di embedding ONNX."""

    def __init__(
        self,
        model_path: str = "models/all-MiniLM-L6-v2.onnx",
        max_length: int = 256,
    ):
        self.model_path = model_path
        self.max_length = max_length
        self.session: Optional[object] = None
        self.dimensions: int = 384  # MiniLM-L6-v2

    def load(self) -> None:
        """Carica il modello ONNX in memoria."""
        if not Path(self.model_path).exists():
            raise FileNotFoundError(
                f"Modello ONNX non trovato: {self.model_path}\n"
                f"Esegui: bash scripts/setup.sh per scaricarlo"
            )

        ort = _load_onnx()
        sess_options = ort.SessionOptions()
        sess_options.intra_op_num_threads = os.cpu_count() or 4
        sess_options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL

        self.session = ort.InferenceSession(
            self.model_path,
            sess_options,
            providers=["CPUExecutionProvider"],
        )
        logger.info(f"Modello caricato: {self.model_path} ({self.dimensions}d)")

    def encode(self, texts: list[str]) -> np.ndarray:
        """
        Calcola embedding per una lista di testi.

        Args:
            texts: Lista di stringhe da codificare

        Returns:
            np.ndarray di shape (len(texts), self.dimensions) con valori float32
        """
        if self.session is None:
            self.load()

        # Tokenizzazione semplificata (placeholder — in produzione usa tokenizers)
        # TODO: integrare tokenizers di HuggingFace per tokenizzazione corretta
        from tokenizers import Tokenizer

        # Per ora: genera embedding casuali normalizzati come placeholder
        # Questo permette di testare la pipeline senza il modello reale
        logger.warning("Usando embedding placeholder — installare modello ONNX per produzione")
        embeddings = np.random.randn(len(texts), self.dimensions).astype(np.float32)
        # Normalizza L2
        norms = np.linalg.norm(embeddings, axis=1, keepdims=True)
        embeddings = embeddings / norms
        return embeddings

    def cosine_similarity(self, vec_a: np.ndarray, vec_b: np.ndarray) -> float:
        """Calcola similarità coseno tra due vettori."""
        dot = np.dot(vec_a, vec_b)
        norm_a = np.linalg.norm(vec_a)
        norm_b = np.linalg.norm(vec_b)
        if norm_a == 0 or norm_b == 0:
            return 0.0
        return float(dot / (norm_a * norm_b))


# Singleton globale
_model: Optional[EmbeddingModel] = None


def get_model() -> EmbeddingModel:
    """Restituisce l'istanza singleton del modello."""
    global _model
    if _model is None:
        _model = EmbeddingModel()
    return _model
