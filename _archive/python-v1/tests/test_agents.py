"""Test per i micro-agenti VARCAVIA."""

import pytest
from agents.classifier_agent import ClassifierAgent


@pytest.mark.asyncio
async def test_classifier_health():
    agent = ClassifierAgent()
    result = await agent.process({"text": "Il paziente presenta sintomi di febbre e diagnosi di influenza"})
    assert result["domain"] == "health"
    assert result["confidence"] > 0.3


@pytest.mark.asyncio
async def test_classifier_climate():
    agent = ClassifierAgent()
    result = await agent.process({"text": "La temperatura media è aumentata di 1.5 gradi, emissioni CO2 in crescita"})
    assert result["domain"] == "climate"


@pytest.mark.asyncio
async def test_classifier_unknown():
    agent = ClassifierAgent()
    result = await agent.process({"text": "Una parola completamente generica"})
    assert result["domain"] == "general"
