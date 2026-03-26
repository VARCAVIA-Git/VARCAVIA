"""
Anomaly Agent — Rilevamento dati anomali o sospetti.
TODO: implementare nella Fase 5.
"""

from typing import Any
from agents.base_agent import BaseAgent


class AnomalyAgent(BaseAgent):
    def __init__(self, **kwargs: Any):
        super().__init__(agent_name="anomaly", **kwargs)

    async def process(self, request: dict[str, Any]) -> dict[str, Any]:
        return {"anomaly_score": 0.0, "is_anomaly": False}
