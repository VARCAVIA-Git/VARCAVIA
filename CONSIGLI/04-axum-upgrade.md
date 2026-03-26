# Upgrade Axum 0.7 → 0.8

## Problema riscontrato
Axum 0.7 usa la sintassi `:param` per i path parameters nelle route.
Axum 0.8+ usa `{param}` (più leggibile, allineato con OpenAPI).

## Quando fare l'upgrade
Quando Axum 0.8 sarà stabile nel workspace, aggiornare:
- `Cargo.toml`: `axum = "0.8"`
- Route: `/:id` → `/{id}`
- Verificare compatibilità tower-http

## Attenzione
Non confondere la sintassi delle route Axum (`:id`) con le format string Rust (`{id}`).
Nei test, le URI usano `format!("/api/v1/data/{data_id}")` — qui `{data_id}` è Rust, non Axum.
