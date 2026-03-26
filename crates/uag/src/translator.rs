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

    // Estrai header dal primo oggetto
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
            // Prova a parsare come numero
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
    fn test_translate_json_to_csv() {
        let data = serde_json::json!([{"a": 1, "b": 2}]);
        let result = translate_format(&data, "json", "csv").unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_unsupported_format() {
        let data = serde_json::json!({});
        let result = translate_format(&data, "xml", "yaml");
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
}
