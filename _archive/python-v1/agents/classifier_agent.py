"""
Classifier Agent — Classificazione automatica dei dati per dominio.
"""

from typing import Any
from agents.base_agent import BaseAgent

# Domini supportati con keywords
DOMAIN_KEYWORDS: dict[str, list[str]] = {
    "health": ["paziente", "diagnosi", "farmaco", "clinico", "medico", "sintomo", "terapia",
               "patient", "diagnosis", "drug", "clinical", "medical", "symptom", "therapy"],
    "climate": ["temperatura", "precipitazioni", "emissioni", "CO2", "clima", "meteo",
                "temperature", "precipitation", "emissions", "climate", "weather"],
    "finance": ["mercato", "azioni", "PIL", "inflazione", "investimento", "borsa",
                "market", "stock", "GDP", "inflation", "investment", "exchange"],
    "science": ["ricerca", "esperimento", "pubblicazione", "peer-review", "ipotesi",
                "research", "experiment", "publication", "hypothesis"],
    "education": ["studente", "corso", "università", "esame", "laurea",
                  "student", "course", "university", "exam", "degree"],
}


class ClassifierAgent(BaseAgent):
    """Agente di classificazione dati per dominio."""

    def __init__(self, **kwargs: Any):
        super().__init__(agent_name="classifier", **kwargs)

    async def process(self, request: dict[str, Any]) -> dict[str, Any]:
        """Classifica un dato per dominio basandosi su keyword matching + embedding."""
        text = request.get("text", "").lower()

        if not text:
            return {"domain": "general", "confidence": 0.0}

        # Keyword matching (veloce, prima approssimazione)
        scores: dict[str, int] = {}
        for domain, keywords in DOMAIN_KEYWORDS.items():
            score = sum(1 for kw in keywords if kw.lower() in text)
            if score > 0:
                scores[domain] = score

        if not scores:
            return {"domain": "general", "confidence": 0.3}

        best_domain = max(scores, key=scores.get)  # type: ignore
        max_possible = len(DOMAIN_KEYWORDS[best_domain])
        confidence = min(scores[best_domain] / max(max_possible * 0.3, 1), 1.0)

        return {
            "domain": best_domain,
            "confidence": round(confidence, 2),
            "all_scores": {k: v / len(DOMAIN_KEYWORDS[k]) for k, v in scores.items()},
        }
