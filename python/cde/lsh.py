"""
Locality-Sensitive Hashing (LSH) — Ricerca near-duplicates veloce.

Implementa MinHash + LSH per trovare dati quasi-identici in tempo O(1) ammortizzato.
Usato nello Stadio 2 della pipeline CDE.
"""

import hashlib
import struct
from typing import Optional

import numpy as np


class MinHashLSH:
    """MinHash LSH per deduplicazione near-duplicate."""

    def __init__(
        self,
        num_perm: int = 128,
        threshold: float = 0.85,
        num_bands: int = 16,
    ):
        """
        Args:
            num_perm: Numero di permutazioni per MinHash
            threshold: Soglia di similarità Jaccard per considerare duplicato
            num_bands: Numero di bande per LSH (più bande = più recall, meno precision)
        """
        self.num_perm = num_perm
        self.threshold = threshold
        self.num_bands = num_bands
        self.rows_per_band = num_perm // num_bands

        # Storage: band_index → {hash → set di document_ids}
        self.bands: list[dict[int, set[str]]] = [
            {} for _ in range(num_bands)
        ]
        # MinHash signatures per documento
        self.signatures: dict[str, np.ndarray] = {}

        # Seed per le permutazioni hash
        self._a = np.random.randint(1, 2**31, size=num_perm, dtype=np.int64)
        self._b = np.random.randint(0, 2**31, size=num_perm, dtype=np.int64)
        self._prime = np.int64(2**31 - 1)

    def _minhash(self, shingles: set[int]) -> np.ndarray:
        """Calcola la signature MinHash di un insieme di shingle."""
        sig = np.full(self.num_perm, np.iinfo(np.int64).max, dtype=np.int64)
        for s in shingles:
            hashes = (self._a * s + self._b) % self._prime
            sig = np.minimum(sig, hashes)
        return sig

    def _shingle(self, text: str, k: int = 3) -> set[int]:
        """Genera k-shingle (come hash interi) da un testo."""
        shingles = set()
        text_lower = text.lower().strip()
        for i in range(len(text_lower) - k + 1):
            shingle = text_lower[i:i + k]
            h = struct.unpack("<i", hashlib.md5(shingle.encode()).digest()[:4])[0]
            shingles.add(h)
        return shingles

    def insert(self, doc_id: str, text: str) -> None:
        """Inserisce un documento nell'indice LSH."""
        shingles = self._shingle(text)
        if not shingles:
            return

        sig = self._minhash(shingles)
        self.signatures[doc_id] = sig

        # Inserisci nelle bande
        for band_idx in range(self.num_bands):
            start = band_idx * self.rows_per_band
            end = start + self.rows_per_band
            band_hash = hash(sig[start:end].tobytes())

            if band_hash not in self.bands[band_idx]:
                self.bands[band_idx][band_hash] = set()
            self.bands[band_idx][band_hash].add(doc_id)

    def query(self, text: str) -> list[tuple[str, float]]:
        """
        Cerca near-duplicates di un testo.

        Returns:
            Lista di (doc_id, estimated_similarity) ordinata per similarità
        """
        shingles = self._shingle(text)
        if not shingles:
            return []

        sig = self._minhash(shingles)
        candidates: set[str] = set()

        # Trova candidati in qualsiasi banda
        for band_idx in range(self.num_bands):
            start = band_idx * self.rows_per_band
            end = start + self.rows_per_band
            band_hash = hash(sig[start:end].tobytes())

            if band_hash in self.bands[band_idx]:
                candidates.update(self.bands[band_idx][band_hash])

        # Calcola similarità esatta per i candidati
        results = []
        for doc_id in candidates:
            stored_sig = self.signatures[doc_id]
            similarity = float(np.mean(sig == stored_sig))
            if similarity >= self.threshold:
                results.append((doc_id, similarity))

        return sorted(results, key=lambda x: x[1], reverse=True)

    def __len__(self) -> int:
        return len(self.signatures)
