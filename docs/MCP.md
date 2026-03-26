# VARCAVIA MCP Server for Claude

VARCAVIA exposes a [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server that gives Claude direct access to cryptographic fact verification.

## Tools

| Tool | Description |
|------|-------------|
| `verify_fact` | Verify a factual claim. Returns Data DNA certificate with BLAKE3+SHA3-512 fingerprints, Ed25519 signature, and reliability score. |
| `search_facts` | Search the VARCAVIA database for similar facts using trigram similarity. |
| `submit_fact` | Submit a new fact to be cryptographically verified and stored. |
| `get_stats` | Get node statistics: total facts, uptime, network health. |

## Setup with Claude Desktop

### 1. Build the MCP server

```bash
cd /path/to/varcavia
cargo build --release --bin varcavia-mcp
```

### 2. Start a VARCAVIA node

```bash
cargo run --release --bin varcavia-node -- --port 8080
```

### 3. Configure Claude Desktop

Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS, `%APPDATA%\Claude\claude_desktop_config.json` on Windows):

```json
{
  "mcpServers": {
    "varcavia": {
      "command": "/path/to/varcavia/target/release/varcavia-mcp",
      "env": {
        "VARCAVIA_URL": "http://127.0.0.1:8080"
      }
    }
  }
}
```

For a remote VARCAVIA node:

```json
{
  "mcpServers": {
    "varcavia": {
      "command": "/path/to/varcavia/target/release/varcavia-mcp",
      "env": {
        "VARCAVIA_URL": "https://varcavia-production.up.railway.app"
      }
    }
  }
}
```

### 4. Use in Claude

Once configured, Claude can verify facts directly:

> "Verify that the speed of light is 299,792,458 m/s"

Claude will call the `verify_fact` tool and show the Data DNA certificate with cryptographic proof.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VARCAVIA_URL` | `http://127.0.0.1:8080` | URL of the VARCAVIA node API |

## Protocol

The MCP server communicates via JSON-RPC 2.0 over stdin/stdout. Example:

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"verify_fact","arguments":{"fact":"Earth diameter is 12742 km"}}}
```

Response:

```json
{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"{\"fact\":\"Earth diameter is 12742 km\",\"score\":0.73,\"data_dna\":{...}}"}]}}
```

## Testing

```bash
# Start the node
cargo run --bin varcavia-node &

# Send a test request
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | cargo run --bin varcavia-mcp
```
