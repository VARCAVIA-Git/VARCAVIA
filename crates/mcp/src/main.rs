//! # VARCAVIA MCP Server
//!
//! Server MCP (Model Context Protocol) minimale che espone il tool `verify_fact`
//! per permettere a Claude di verificare fatti tramite VARCAVIA.
//!
//! Comunicazione via JSON-RPC su stdin/stdout (standard MCP).
//! Richiede un nodo VARCAVIA attivo su localhost:8080.

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

/// URL base del nodo VARCAVIA locale.
const VARCAVIA_API: &str = "http://127.0.0.1:8080";

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

// === MCP protocol types ===

fn server_info() -> serde_json::Value {
    serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
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
                "description": "Verify a factual claim using VARCAVIA's cryptographic verification protocol. Returns a Data DNA certificate with dual fingerprints (BLAKE3 + SHA3-512), Ed25519 signature, and reliability score.",
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
            }
        ]
    })
}

/// Chiama l'API VARCAVIA per verificare un fatto.
async fn call_verify(fact: &str) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/api/v1/verify?fact={}",
        VARCAVIA_API,
        urlencoded(fact)
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Connessione al nodo VARCAVIA fallita: {e}"))?;

    let body = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Risposta non valida: {e}"))?;

    Ok(body)
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

fn make_response(id: serde_json::Value, result: serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(result),
        error: None,
    }
}

fn make_error(id: serde_json::Value, code: i64, message: String) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(JsonRpcError { code, message }),
    }
}

fn handle_tools_call(params: &serde_json::Value) -> Result<String, String> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing tool name")?;

    if name != "verify_fact" {
        return Err(format!("Tool sconosciuto: {name}"));
    }

    let args = params.get("arguments").unwrap_or(&serde_json::Value::Null);
    let fact = args
        .get("fact")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'fact' argument")?;

    Ok(fact.to_string())
}

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
                let resp = make_error(
                    serde_json::Value::Null,
                    -32700,
                    format!("Parse error: {e}"),
                );
                let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };

        let id = req.id.clone().unwrap_or(serde_json::Value::Null);

        let resp = match req.method.as_str() {
            "initialize" => make_response(id, server_info()),

            "notifications/initialized" => continue, // no response needed

            "tools/list" => make_response(id, tools_list()),

            "tools/call" => {
                match handle_tools_call(&req.params) {
                    Ok(fact) => {
                        match call_verify(&fact).await {
                            Ok(result) => {
                                // Format as MCP tool result
                                let text = serde_json::to_string_pretty(&result)
                                    .unwrap_or_else(|_| "{}".into());
                                make_response(
                                    id,
                                    serde_json::json!({
                                        "content": [
                                            {
                                                "type": "text",
                                                "text": text
                                            }
                                        ]
                                    }),
                                )
                            }
                            Err(e) => make_response(
                                id,
                                serde_json::json!({
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": format!("Error: {e}")
                                        }
                                    ],
                                    "isError": true
                                }),
                            ),
                        }
                    }
                    Err(e) => make_error(id, -32602, e),
                }
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
    fn test_tools_list() {
        let list = tools_list();
        let tools = list["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "verify_fact");
    }

    #[test]
    fn test_handle_tools_call_ok() {
        let params = serde_json::json!({
            "name": "verify_fact",
            "arguments": { "fact": "test fact" }
        });
        assert_eq!(handle_tools_call(&params).unwrap(), "test fact");
    }

    #[test]
    fn test_handle_tools_call_unknown() {
        let params = serde_json::json!({ "name": "unknown" });
        assert!(handle_tools_call(&params).is_err());
    }

    #[test]
    fn test_handle_tools_call_missing_fact() {
        let params = serde_json::json!({
            "name": "verify_fact",
            "arguments": {}
        });
        assert!(handle_tools_call(&params).is_err());
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
}
