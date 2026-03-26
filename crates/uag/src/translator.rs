//! Universal Format Translator — Conversione tra formati (JSON ↔ CSV ↔ XML).

use crate::{UagError, Result};

/// Converte un dato da un formato a un altro.
pub fn translate_format(
    data: &serde_json::Value,
    from_format: &str,
    to_format: &str,
) -> Result<String> {
    match (from_format, to_format) {
        ("json", "csv") => json_to_csv(data),
        ("csv", "json") => csv_to_json(data),
        ("json", "xml") => json_to_xml(data),
        ("xml", "json") => xml_to_json(data),
        ("json", "json") => serde_json::to_string_pretty(data)
            .map_err(|e| UagError::TranslationFailed(e.to_string())),
        _ => Err(UagError::UnsupportedFormat(format!(
            "{from_format} → {to_format}"
        ))),
    }
}

/// Converte JSON (array di oggetti) in CSV.
fn json_to_csv(data: &serde_json::Value) -> Result<String> {
    let arr = data
        .as_array()
        .ok_or_else(|| UagError::TranslationFailed("Input deve essere un array JSON".into()))?;

    if arr.is_empty() {
        return Ok(String::new());
    }

    let first = arr[0]
        .as_object()
        .ok_or_else(|| UagError::TranslationFailed("Array deve contenere oggetti".into()))?;
    let headers: Vec<&String> = first.keys().collect();

    let mut csv = headers
        .iter()
        .map(|h| h.as_str())
        .collect::<Vec<_>>()
        .join(",");
    csv.push('\n');

    for item in arr {
        if let Some(obj) = item.as_object() {
            let row: Vec<String> = headers
                .iter()
                .map(|h| {
                    obj.get(*h)
                        .map(|v| match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .unwrap_or_default()
                })
                .collect();
            csv.push_str(&row.join(","));
            csv.push('\n');
        }
    }

    Ok(csv)
}

/// Converte CSV (come stringa in un campo JSON) in JSON array.
fn csv_to_json(data: &serde_json::Value) -> Result<String> {
    let csv_str = data
        .as_str()
        .ok_or_else(|| UagError::TranslationFailed("Input deve essere una stringa CSV".into()))?;

    let mut lines = csv_str.lines();
    let headers: Vec<&str> = lines
        .next()
        .ok_or_else(|| UagError::TranslationFailed("CSV vuoto".into()))?
        .split(',')
        .map(|s| s.trim())
        .collect();

    let mut result = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        let mut obj = serde_json::Map::new();
        for (i, header) in headers.iter().enumerate() {
            let value = values.get(i).unwrap_or(&"");
            if let Ok(n) = value.parse::<f64>() {
                obj.insert(header.to_string(), serde_json::json!(n));
            } else {
                obj.insert(header.to_string(), serde_json::json!(value));
            }
        }
        result.push(serde_json::Value::Object(obj));
    }

    serde_json::to_string_pretty(&result)
        .map_err(|e| UagError::TranslationFailed(e.to_string()))
}

/// Converte JSON in XML.
fn json_to_xml(data: &serde_json::Value) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    json_value_to_xml(data, "root", &mut xml, 0);
    Ok(xml)
}

fn json_value_to_xml(value: &serde_json::Value, tag: &str, out: &mut String, indent: usize) {
    let pad = "  ".repeat(indent);
    match value {
        serde_json::Value::Object(map) => {
            out.push_str(&format!("{pad}<{tag}>\n"));
            for (key, val) in map {
                json_value_to_xml(val, key, out, indent + 1);
            }
            out.push_str(&format!("{pad}</{tag}>\n"));
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                json_value_to_xml(item, tag, out, indent);
            }
        }
        serde_json::Value::String(s) => {
            let escaped = xml_escape(s);
            out.push_str(&format!("{pad}<{tag}>{escaped}</{tag}>\n"));
        }
        serde_json::Value::Number(n) => {
            out.push_str(&format!("{pad}<{tag}>{n}</{tag}>\n"));
        }
        serde_json::Value::Bool(b) => {
            out.push_str(&format!("{pad}<{tag}>{b}</{tag}>\n"));
        }
        serde_json::Value::Null => {
            out.push_str(&format!("{pad}<{tag}/>\n"));
        }
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Converte semplice XML (passato come stringa in un campo JSON) in JSON.
fn xml_to_json(data: &serde_json::Value) -> Result<String> {
    let xml_str = data
        .as_str()
        .ok_or_else(|| UagError::TranslationFailed("Input deve essere una stringa XML".into()))?;

    let value = parse_xml_simple(xml_str)?;
    serde_json::to_string_pretty(&value)
        .map_err(|e| UagError::TranslationFailed(e.to_string()))
}

/// Parser XML minimale — gestisce elementi semplici con testo.
fn parse_xml_simple(xml: &str) -> Result<serde_json::Value> {
    let mut result = serde_json::Map::new();
    let content = xml.trim();

    // Skip XML declaration
    let content = if let Some(rest) = content.strip_prefix("<?") {
        rest.split_once("?>")
            .map(|(_, after)| after.trim())
            .unwrap_or(content)
    } else {
        content
    };

    parse_xml_elements(content, &mut result)?;

    Ok(serde_json::Value::Object(result))
}

fn parse_xml_elements(content: &str, map: &mut serde_json::Map<String, serde_json::Value>) -> Result<()> {
    let mut remaining = content.trim();

    while !remaining.is_empty() {
        // Find opening tag
        let Some(tag_start) = remaining.find('<') else {
            break;
        };

        // Skip if it's a closing tag at top level
        if remaining[tag_start + 1..].starts_with('/') {
            break;
        }

        // Self-closing tag
        if let Some(end) = remaining[tag_start..].find("/>") {
            let tag_name = &remaining[tag_start + 1..tag_start + end].trim();
            map.insert(tag_name.to_string(), serde_json::Value::Null);
            remaining = &remaining[tag_start + end + 2..];
            continue;
        }

        let Some(tag_end) = remaining[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_end;
        let tag_name = remaining[tag_start + 1..tag_end].trim().to_string();

        // Find closing tag
        let close_tag = format!("</{tag_name}>");
        let Some(close_pos) = remaining.find(&close_tag) else {
            break;
        };

        let inner = &remaining[tag_end + 1..close_pos];

        // Check if inner content has child elements
        if inner.contains('<') {
            let mut child_map = serde_json::Map::new();
            parse_xml_elements(inner, &mut child_map)?;
            map.insert(tag_name, serde_json::Value::Object(child_map));
        } else {
            // Leaf text node
            let text = inner.trim();
            if let Ok(n) = text.parse::<f64>() {
                map.insert(tag_name, serde_json::json!(n));
            } else {
                map.insert(tag_name, serde_json::json!(text));
            }
        }

        remaining = &remaining[close_pos + close_tag.len()..];
        remaining = remaining.trim();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_csv() {
        let data = serde_json::json!([
            {"city": "Roma", "temp": 22},
            {"city": "Milano", "temp": 18}
        ]);
        let csv = json_to_csv(&data).unwrap();
        assert!(csv.contains("city"));
        assert!(csv.contains("Roma"));
        assert!(csv.contains("Milano"));
    }

    #[test]
    fn test_csv_to_json() {
        let csv = serde_json::json!("city,temp\nRoma,22\nMilano,18");
        let json_str = csv_to_json(&csv).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["city"], "Roma");
    }

    #[test]
    fn test_json_to_xml() {
        let data = serde_json::json!({"city": "Roma", "temp": 22});
        let xml = json_to_xml(&data).unwrap();
        assert!(xml.contains("<?xml"));
        assert!(xml.contains("<city>Roma</city>"));
        assert!(xml.contains("<temp>22</temp>"));
    }

    #[test]
    fn test_json_array_to_xml() {
        let data = serde_json::json!({
            "items": [
                {"name": "a"},
                {"name": "b"}
            ]
        });
        let xml = json_to_xml(&data).unwrap();
        assert!(xml.contains("<name>a</name>"));
        assert!(xml.contains("<name>b</name>"));
    }

    #[test]
    fn test_xml_to_json() {
        let xml = serde_json::json!("<root><city>Roma</city><temp>22</temp></root>");
        let json_str = xml_to_json(&xml).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["root"]["city"], "Roma");
        assert_eq!(parsed["root"]["temp"], 22.0);
    }

    #[test]
    fn test_xml_with_declaration() {
        let xml = serde_json::json!("<?xml version=\"1.0\"?><data><val>42</val></data>");
        let json_str = xml_to_json(&xml).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["data"]["val"], 42.0);
    }

    #[test]
    fn test_translate_json_to_csv() {
        let data = serde_json::json!([{"a": 1, "b": 2}]);
        let result = translate_format(&data, "json", "csv").unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_translate_json_to_xml() {
        let data = serde_json::json!({"key": "value"});
        let result = translate_format(&data, "json", "xml").unwrap();
        assert!(result.contains("<key>value</key>"));
    }

    #[test]
    fn test_unsupported_format() {
        let data = serde_json::json!({});
        let result = translate_format(&data, "yaml", "toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_json_identity() {
        let data = serde_json::json!({"key": "value"});
        let result = translate_format(&data, "json", "json").unwrap();
        assert!(result.contains("value"));
    }

    #[test]
    fn test_empty_array_to_csv() {
        let data = serde_json::json!([]);
        let csv = json_to_csv(&data).unwrap();
        assert!(csv.is_empty());
    }

    #[test]
    fn test_xml_escape() {
        let data = serde_json::json!({"msg": "a < b & c > d"});
        let xml = json_to_xml(&data).unwrap();
        assert!(xml.contains("a &lt; b &amp; c &gt; d"));
    }
}
