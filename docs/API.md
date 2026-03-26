# VARCAVIA — Documentazione API

## Base URL

```
http://localhost:8080/api/v1
```

## Autenticazione

Tutti gli endpoint di scrittura richiedono una firma Ed25519 nell'header:
```
X-Varcavia-Signature: <firma_hex>
X-Varcavia-PublicKey: <chiave_pubblica_hex>
```

Gli endpoint di lettura sono pubblici.

## Endpoints

### Dati

#### `POST /data` — Inserisci un dato
```json
// Request
{
  "content": "La temperatura a Roma è 22°C",
  "domain": "climate",
  "content_type": "text/plain",
  "metadata": {
    "source_name": "sensore-roma-01",
    "location": { "lat": 41.9028, "lon": 12.4964 }
  }
}

// Response 201
{
  "id": "a1b2c3d4e5f6...",
  "ddna": { ... },
  "score": 0.72,
  "status": "confirmed"
}
```

#### `GET /data/{id}` — Recupera un dato
```json
// Response 200
{
  "id": "a1b2c3d4e5f6...",
  "content": "La temperatura a Roma è 22°C",
  "ddna": { ... },
  "score": 0.72,
  "created_at": "2026-03-25T10:30:00Z"
}
```

#### `GET /data/{id}/dna` — Solo il Data DNA
```json
// Response 200
{
  "version": 1,
  "fingerprint": { "blake3": "...", "sha3_512": "...", "content_size": 42 },
  "source": { "public_key": "...", "reputation_score": 0.85 },
  "temporal": { "timestamp_us": 1711360200000000 },
  "custody_chain": [ ... ],
  "integrity_hash": "..."
}
```

#### `POST /data/query` — Query semantica
```json
// Request
{
  "query": "temperatura città italiane",
  "domain": "climate",
  "min_score": 0.5,
  "limit": 10
}

// Response 200
{
  "results": [
    { "id": "...", "content": "...", "score": 0.85, "similarity": 0.92 }
  ],
  "total": 42
}
```

#### `POST /data/verify` — Verifica autenticità
```json
// Request
{ "content": "La temperatura a Roma è 22°C", "ddna_hash": "a1b2c3..." }

// Response 200
{
  "verified": true,
  "fingerprint_match": true,
  "signature_valid": true,
  "chain_valid": true,
  "score": 0.72
}
```

### Nodo

#### `GET /node/status` — Stato del nodo
```json
{
  "node_id": "...",
  "name": "varcavia-dev-01",
  "version": "0.1.0",
  "uptime_secs": 3600,
  "data_count": 1234,
  "peers_connected": 5,
  "storage_used_mb": 256
}
```

#### `GET /node/peers` — Nodi connessi
```json
{
  "peers": [
    { "node_id": "...", "address": "127.0.0.1:7701", "reputation": 0.9, "latency_ms": 2 }
  ]
}
```

### Rete

#### `GET /network/health` — Salute della rete
```json
{
  "total_nodes": 5,
  "active_nodes": 5,
  "avg_latency_ms": 3,
  "consensus_success_rate": 0.98,
  "data_throughput_per_sec": 1500
}
```

### Traduzione Formati

#### `POST /translate` — Converti formato
```json
// Request
{
  "data": { "name": "Roma", "temp": 22 },
  "from_format": "json",
  "to_format": "csv"
}

// Response 200
{
  "result": "name,temp\nRoma,22",
  "format": "csv"
}
```

## Codici di Errore

| Codice | Significato |
|--------|-------------|
| 400 | Richiesta malformata |
| 401 | Firma non valida o mancante |
| 404 | Dato non trovato |
| 409 | Dato duplicato |
| 429 | Rate limit superato |
| 500 | Errore interno |
