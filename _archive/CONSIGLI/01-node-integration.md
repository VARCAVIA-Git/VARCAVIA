# Integrazione node con storage e network

Il binary `varcavia-node` ha storage.rs e network.rs implementati ma non ancora collegati a main.rs.

## Prossimi passi (Fase 2)

1. **main.rs**: al boot, aprire lo Storage e avviare il NetworkManager
2. **CLI commands**: aggiungere `insert`, `query`, `peers` ai subcommand
3. **Integrazione CDE**: quando un dato viene inserito via CLI/API, farlo passare per la pipeline CDE
4. **Integrazione ARC**: quando un dato supera la CDE, avviare il consenso ARC con i peer connessi
5. **Status command**: mostrare dati reali (count dati, peer connessi, uptime)

## Note tecniche
- storage.rs usa `sled` (alternativa leggera a RocksDB, compila ovunque)
- network.rs usa TCP puro con length-prefixed JSON messages
- Per la Fase 2: migrare a libp2p per discovery automatico (mDNS), encryption (noise), multiplexing (yamux)
