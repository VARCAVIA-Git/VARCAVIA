# Miglioramenti UAG (API Gateway)

## Stato attuale
- REST endpoints implementati come scaffold (risposte statiche/placeholder)
- GraphQL è un placeholder
- Translator supporta JSON↔CSV

## Miglioramenti suggeriti

### Breve termine
1. **Shared state**: usare `axum::extract::State` con un `Arc<AppState>` che contenga Storage + CDE Pipeline
2. **Rate limiting reale**: implementare con `tower::limit::RateLimit` o `governor` crate
3. **Auth**: verificare header `X-Varcavia-Signature` con Ed25519 come da spec
4. **Error handling**: usare il pattern `Result<Json<T>, ApiError>` in tutti gli handler

### Medio termine
5. **GraphQL**: usare `async-graphql` con schema derivato dalle struct esistenti
6. **WebSocket**: endpoint `/ws` per stream real-time di dati validati
7. **Translator**: aggiungere supporto XML (con `quick-xml` crate)
8. **OpenAPI**: generare spec con `utoipa` per documentazione automatica
