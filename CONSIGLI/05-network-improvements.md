# Miglioramenti rete P2P

## Stato attuale (Fase 3)
- TCP puro con length-prefixed JSON
- Peer list statica da --peers flag
- Consenso sincrono con timeout 500ms
- Replicazione automatica su voto approve

## Miglioramenti suggeriti

### Breve termine
1. **Peer discovery automatico**: implementare gossip protocol per condividere la lista peer
2. **Heartbeat**: ping periodico ai peer per rilevare disconnessioni
3. **Riconnessione**: tentare riconnessione automatica ai peer persi
4. **Peer exchange nel Pong**: includere la lista di peer noti nella risposta Pong

### Medio termine
5. **libp2p migration**: sostituire TCP puro con libp2p per:
   - mDNS discovery locale
   - Noise protocol encryption
   - Yamux multiplexing
   - NAT traversal
6. **Comitato di validazione**: selezionare solo N peer per dominio (committee.rs)
7. **Bandwidth limiting**: limitare la banda usata per la replicazione

### Nota architetturale
I messaggi di rete sono in `vtp/messages.rs` per essere condivisi tra UAG (consensus) e node (network handler). Questo è pulito ma significa che VTP ha una dipendenza da serde_json. In futuro, valutare se usare MessagePack anche per i messaggi P2P (più compatto).
