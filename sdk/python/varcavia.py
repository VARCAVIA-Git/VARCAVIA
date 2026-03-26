"""
VARCAVIA Python SDK — Minimal client for the VERIT Protocol.

Usage:
    from varcavia import Varcavia

    v = Varcavia("http://localhost:8080")
    result = v.verify("Earth diameter is 12742 km")
    print(result["score"])

Requires: requests (pip install requests)
"""

import requests


class Varcavia:
    """Client for the VARCAVIA VERIT Protocol API."""

    def __init__(self, base_url: str = "http://localhost:8080"):
        self.base_url = base_url.rstrip("/")

    def verify(self, fact: str) -> dict:
        """Verify a fact and get its Data DNA + score.

        Args:
            fact: The factual claim to verify.

        Returns:
            Dict with keys: fact, status, data_dna, score, verification_count, duplicate
        """
        r = requests.get(
            f"{self.base_url}/api/v1/verify",
            params={"fact": fact},
        )
        r.raise_for_status()
        return r.json()

    def submit(self, content: str, domain: str = "general", source: str = "python-sdk") -> str:
        """Submit data to the network and get its ID.

        Args:
            content: The data content.
            domain: Data domain (climate, health, finance, science, general).
            source: Source identifier.

        Returns:
            The data ID (blake3 hex).
        """
        r = requests.post(
            f"{self.base_url}/api/v1/data",
            json={"content": content, "domain": domain, "source": source},
        )
        r.raise_for_status()
        return r.json()["id"]

    def get(self, data_id: str) -> dict:
        """Get data by ID.

        Args:
            data_id: The blake3 hex ID.

        Returns:
            Dict with keys: id, content, domain, score
        """
        r = requests.get(f"{self.base_url}/api/v1/data/{data_id}")
        r.raise_for_status()
        return r.json()

    def get_dna(self, data_id: str) -> dict:
        """Get the full Data DNA for a datum."""
        r = requests.get(f"{self.base_url}/api/v1/data/{data_id}/dna")
        r.raise_for_status()
        return r.json()

    def query(self, domain: str = None, limit: int = 20) -> list:
        """Query data by domain."""
        body = {"query": "", "limit": limit}
        if domain:
            body["domain"] = domain
        r = requests.post(f"{self.base_url}/api/v1/data/query", json=body)
        r.raise_for_status()
        return r.json()

    def stats(self) -> dict:
        """Get node statistics."""
        r = requests.get(f"{self.base_url}/api/v1/stats")
        r.raise_for_status()
        return r.json()

    def health(self) -> dict:
        """Health check."""
        r = requests.get(f"{self.base_url}/health")
        r.raise_for_status()
        return r.json()


if __name__ == "__main__":
    v = Varcavia()
    print("Verifying fact...")
    result = v.verify("Earth diameter is 12742 km")
    print(f"  Status: {result['status']}")
    print(f"  Score: {result['score']:.2%}")
    print(f"  ID: {result['data_dna']['id']}")
    print(f"  BLAKE3: {result['data_dna']['fingerprint']['blake3']}")
