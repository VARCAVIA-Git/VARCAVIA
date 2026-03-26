"""
Base Agent — Classe base per tutti i micro-agenti AI di VARCAVIA.

Ogni agente:
- Si connette al nodo Rust via JSON-RPC su socket Unix
- Esegue un loop di lavoro (process) a intervalli regolari
- Comunica risultati al nodo Rust
"""

import asyncio
import json
import logging
import os
import socket
from abc import ABC, abstractmethod
from typing import Any

logger = logging.getLogger("varcavia.agent")


class BaseAgent(ABC):
    """Classe base per micro-agenti VARCAVIA."""

    def __init__(self, agent_name: str, node_socket_path: str | None = None):
        self.agent_name = agent_name
        self.socket_path = node_socket_path or os.environ.get(
            "VARCAVIA_AGENT_SOCKET", "/tmp/varcavia-agent.sock"
        )
        self.running = False
        self._request_id = 0
        logger.info(f"Agent {agent_name} inizializzato, socket: {self.socket_path}")

    @abstractmethod
    async def process(self, request: dict[str, Any]) -> dict[str, Any]:
        """Processa una richiesta dal nodo Rust. Implementare nelle sottoclassi."""
        ...

    async def send_to_node(self, method: str, params: dict[str, Any]) -> dict[str, Any]:
        """Invia una richiesta JSON-RPC al nodo Rust."""
        self._request_id += 1
        message = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": self._request_id,
        }
        # TODO: implementare comunicazione socket Unix reale
        logger.debug(f"→ {method}: {json.dumps(params)[:200]}")
        return {"jsonrpc": "2.0", "result": {}, "id": self._request_id}

    async def run(self, interval_secs: float = 10.0) -> None:
        """Loop principale dell'agente."""
        self.running = True
        logger.info(f"Agent {self.agent_name} avviato, intervallo: {interval_secs}s")

        while self.running:
            try:
                # Poll per nuove richieste dal nodo
                response = await self.send_to_node("agent.poll", {
                    "agent_name": self.agent_name,
                })
                if response.get("result", {}).get("has_work"):
                    result = await self.process(response["result"]["request"])
                    await self.send_to_node("agent.result", {
                        "agent_name": self.agent_name,
                        "result": result,
                    })
            except Exception as e:
                logger.error(f"Errore in {self.agent_name}: {e}")

            await asyncio.sleep(interval_secs)

    def stop(self) -> None:
        """Ferma l'agente."""
        self.running = False
        logger.info(f"Agent {self.agent_name} fermato")
