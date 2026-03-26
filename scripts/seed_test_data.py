#!/usr/bin/env python3
"""
VARCAVIA — Popola il nodo con dati di test.

Uso: python scripts/seed_test_data.py [--url http://localhost:8080] [--count 100]
"""

import argparse
import json
import random
import sys
import urllib.request

SAMPLE_DATA = [
    {"content": "La temperatura a Roma è 22°C con cielo sereno", "domain": "climate"},
    {"content": "Il paziente presenta febbre a 38.5°C e tosse secca", "domain": "health"},
    {"content": "Il PIL italiano è cresciuto dello 0.7% nel Q4 2025", "domain": "finance"},
    {"content": "Nuovo studio su Nature conferma l'efficacia del vaccino", "domain": "science"},
    {"content": "L'Università di Bologna apre 15 nuovi corsi di laurea", "domain": "education"},
    {"content": "Allerta meteo: forti precipitazioni previste in Liguria", "domain": "climate"},
    {"content": "Pressione sanguigna 120/80, valori nella norma", "domain": "health"},
    {"content": "Bitcoin supera i 100.000 USD per la prima volta", "domain": "finance"},
    {"content": "Emissioni di CO2 in calo del 3% in Europa nel 2025", "domain": "climate"},
    {"content": "Terremoto di magnitudo 4.2 registrato in Irpinia", "domain": "emergency"},
]


def seed(url: str, count: int) -> None:
    """Inserisce dati di test nel nodo VARCAVIA."""
    endpoint = f"{url}/api/v1/data"
    success = 0
    errors = 0

    for i in range(count):
        sample = random.choice(SAMPLE_DATA)
        data = {
            "content": f"{sample['content']} [test-{i:04d}]",
            "domain": sample["domain"],
            "source": f"test-seed-{random.randint(1, 10):02d}",
        }

        try:
            req = urllib.request.Request(
                endpoint,
                data=json.dumps(data).encode(),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=5) as resp:
                if resp.status == 201:
                    success += 1
                else:
                    errors += 1
        except Exception as e:
            errors += 1
            if errors == 1:
                print(f"Errore: {e}")
                print(f"Il nodo è attivo su {url}?")

        if (i + 1) % 10 == 0:
            print(f"  Inseriti: {success}/{i+1} (errori: {errors})")

    print(f"\nCompletato: {success} inseriti, {errors} errori su {count} totali")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Seed VARCAVIA con dati di test")
    parser.add_argument("--url", default="http://localhost:8080", help="URL del nodo")
    parser.add_argument("--count", type=int, default=100, help="Numero di dati da inserire")
    args = parser.parse_args()
    seed(args.url, args.count)
