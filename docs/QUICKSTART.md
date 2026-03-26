# Quick Start — 5 Minutes to Your First Verified Fact

## Prerequisites

- Rust 1.78+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- That's it. No Docker, no cloud, no GPU.

## Step 1: Build (30 seconds)

```bash
git clone https://github.com/VARCAVIA-Git/VARCAVIA.git
cd VARCAVIA
cargo build --bin varcavia-node
```

## Step 2: Run (instant)

```bash
cargo run --bin varcavia-node -- --port 8080
```

You'll see:
```
INFO varcavia_node: VARCAVIA Node v0.1.0
INFO varcavia_node: Node ID: 9c8f1460...4d7bf51c
INFO varcavia_uag::server: UAG server avviato su 127.0.0.1:8080
```

## Step 3: Verify a Fact (1 second)

```bash
curl "http://localhost:8080/api/v1/verify?fact=Earth+diameter+is+12742+km"
```

Response:
```json
{
  "fact": "Earth diameter is 12742 km",
  "status": "verified",
  "score": 0.73,
  "data_dna": {
    "id": "bbf3d88a...",
    "fingerprint": {
      "blake3": "bbf3d88a...",
      "sha3_512": "631cc1a6..."
    }
  }
}
```

That's it. Your fact now has a cryptographic identity.

## Step 4: Open the Dashboard

Navigate to [http://localhost:8080](http://localhost:8080) in your browser.

You'll see the verification interface — paste a fact, click Verify, see the Data DNA.

## Step 5: Try the SDKs

### Python
```python
from sdk.python.varcavia import Varcavia

v = Varcavia("http://localhost:8080")
result = v.verify("Water boils at 100 degrees celsius")
print(f"Score: {result['score']:.0%}")
```

### JavaScript
```javascript
import { Varcavia } from './sdk/javascript/varcavia.js';

const v = new Varcavia('http://localhost:8080');
const result = await v.verify('Speed of light is 299792458 m/s');
console.log(`Score: ${(result.score * 100).toFixed(0)}%`);
```

## Step 6: Multi-Node Network (optional)

```bash
bash scripts/run_local_network.sh 3
```

This starts 3 nodes that automatically:
- Discover each other
- Validate data via ARC consensus
- Replicate verified facts across the network

## What's Next?

- Read the [VERIT Protocol Spec](VERITPROTOCOL.md) for the full technical details
- Check the [API Reference](../README.md#api-reference) for all endpoints
- See the [FAQ](FAQ.md) for common questions
- Read [CONTRIBUTING.md](../CONTRIBUTING.md) to contribute
