//! # VARCAVIA MCP Server
//!
//! Server MCP (Model Context Protocol) che espone VARCAVIA come tool per Claude.
//! Tools: verify_fact, search_facts, submit_fact, get_stats.
//!
//! Comunicazione via JSON-RPC su stdin/stdout (standard MCP).
//! Richiede un nodo VARCAVIA attivo (default: http://127.0.0.1:8080).

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

fn api_base() -> String {
    std::env::var("VARCAVIA_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".into())
}

// === JSON-RPC types ===

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

// === MCP protocol ===

fn server_info() -> serde_json::Value {
    serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "varcavia",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn tools_list() -> serde_json::Value {
    serde_json::json!({
        "tools": [
            {
                "name": "verify_fact",
                "description": "Verify a factual claim using VARCAVIA's cryptographic verification protocol. Returns a Data DNA certificate with BLAKE3+SHA3-512 fingerprints, Ed25519 signature, and reliability score. If the fact was already verified, returns the existing record with verification count.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "fact": {
                            "type": "string",
                            "description": "The factual claim to verify (e.g. 'Earth diameter is 12742 km')"
                        }
                    },
                    "required": ["fact"]
                }
            },
            {
                "name": "search_facts",
                "description": "Search the VARCAVIA database for facts similar to a query using trigram similarity matching. Returns the most relevant verified facts with their similarity scores.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query (e.g. 'Earth radius', 'water boiling point')"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum results to return (default: 5, max: 20)",
                            "default": 5
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "submit_fact",
                "description": "Submit a new factual claim to the VARCAVIA network. The fact will be cryptographically fingerprinted, validated through a 6-stage Clean Data Engine, and stored with a reliability score.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The factual content to submit"
                        },
                        "domain": {
                            "type": "string",
                            "description": "Knowledge domain (science, geography, health, climate, technology, general)",
                            "default": "general"
                        }
                    },
                    "required": ["content"]
                }
            },
            {
                "name": "get_stats",
                "description": "Get current statistics from the VARCAVIA node including total verified facts, uptime, and network health.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        ]
    })
}

// === HTTP client helpers ===

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
}

async fn api_get(path: &str) -> Result<serde_json::Value, String> {
    let url = format!("{}{path}", api_base());
    client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Invalid response: {e}"))
}

async fn api_post(path: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
    let url = format!("{}{path}", api_base());
    client()
        .post(&url)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Invalid response: {e}"))
}

/// URL-encode minimale per query string.
fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

// === Tool dispatch ===

async fn handle_tool(name: &str, args: &serde_json::Value) -> Result<serde_json::Value, String> {
    match name {
        "verify_fact" => {
            let fact = args.get("fact").and_then(|v| v.as_str())
                .ok_or("Missing 'fact' argument")?;
            api_get(&format!("/api/v1/verify?fact={}", urlencoded(fact))).await
        }
        "search_facts" => {
            let query = args.get("query").and_then(|v| v.as_str())
                .ok_or("Missing 'query' argument")?;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5).min(20);
            api_get(&format!("/api/v1/search?q={}&limit={limit}", urlencoded(query))).await
        }
        "submit_fact" => {
            let content = args.get("content").and_then(|v| v.as_str())
                .ok_or("Missing 'content' argument")?;
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("general");
            api_post("/api/v1/data", &serde_json::json!({
                "content": content,
                "domain": domain,
                "source": "mcp-claude"
            })).await
        }
        "get_stats" => {
            let status = api_get("/api/v1/node/status").await?;
            let metrics = api_get("/api/v1/metrics").await?;
            let health = api_get("/api/v1/network/health").await?;
            Ok(serde_json::json!({
                "node": status,
                "metrics": metrics,
                "network": health,
            }))
        }
        _ => Err(format!("Unknown tool: {name}")),
    }
}

// === Response helpers ===

fn make_response(id: serde_json::Value, result: serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
}

fn make_error(id: serde_json::Value, code: i64, message: String) -> JsonRpcResponse {
    JsonRpcResponse { jsonrpc: "2.0".into(), id, result: None, error: Some(JsonRpcError { code, message }) }
}

fn tool_result(id: serde_json::Value, data: Result<serde_json::Value, String>) -> JsonRpcResponse {
    match data {
        Ok(val) => {
            let text = serde_json::to_string_pretty(&val).unwrap_or_else(|_| "{}".into());
            make_response(id, serde_json::json!({
                "content": [{ "type": "text", "text": text }]
            }))
        }
        Err(e) => make_response(id, serde_json::json!({
            "content": [{ "type": "text", "text": format!("Error: {e}") }],
            "isError": true
        })),
    }
}

// === Main loop ===

#[tokio::main]
async fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            _ => continue,
        };

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = make_error(serde_json::Value::Null, -32700, format!("Parse error: {e}"));
                let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };

        let id = req.id.clone().unwrap_or(serde_json::Value::Null);

        let resp = match req.method.as_str() {
            "initialize" => make_response(id, server_info()),
            "notifications/initialized" => continue,
            "tools/list" => make_response(id, tools_list()),
            "tools/call" => {
                let name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = req.params.get("arguments").cloned().unwrap_or(serde_json::Value::Null);
                tool_result(id, handle_tool(name, &args).await)
            }
            _ => make_error(id, -32601, format!("Method not found: {}", req.method)),
        };

        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
        let _ = stdout.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencoded() {
        assert_eq!(urlencoded("hello world"), "hello+world");
        assert_eq!(urlencoded("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn test_server_info() {
        let info = server_info();
        assert_eq!(info["serverInfo"]["name"], "varcavia");
        assert!(info["capabilities"]["tools"].is_object());
    }

    #[test]
    fn test_tools_list_has_four_tools() {
        let list = tools_list();
        let tools = list["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"verify_fact"));
        assert!(names.contains(&"search_facts"));
        assert!(names.contains(&"submit_fact"));
        assert!(names.contains(&"get_stats"));
    }

    #[test]
    fn test_make_response() {
        let resp = make_response(serde_json::json!(1), serde_json::json!({"ok": true}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_make_error() {
        let resp = make_error(serde_json::json!(1), -32601, "not found".into());
        assert!(resp.result.is_none());
        assert_eq!(resp.error.as_ref().unwrap().code, -32601);
    }

    #[test]
    fn test_tool_result_ok() {
        let resp = tool_result(serde_json::json!(1), Ok(serde_json::json!({"score": 0.9})));
        let content = resp.result.unwrap();
        assert!(content["content"][0]["text"].as_str().unwrap().contains("0.9"));
    }

    #[test]
    fn test_tool_result_err() {
        let resp = tool_result(serde_json::json!(1), Err("connection failed".into()));
        let content = resp.result.unwrap();
        assert!(content["isError"].as_bool().unwrap());
        assert!(content["content"][0]["text"].as_str().unwrap().contains("connection failed"));
    }

    #[tokio::test]
    async fn test_handle_tool_unknown() {
        let result = handle_tool("nonexistent", &serde_json::Value::Null).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_handle_tool_verify_missing_arg() {
        let result = handle_tool("verify_fact", &serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_tool_search_missing_arg() {
        let result = handle_tool("search_facts", &serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_tool_submit_missing_arg() {
        let result = handle_tool("submit_fact", &serde_json::json!({})).await;
        assert!(result.is_err());
    }
}
