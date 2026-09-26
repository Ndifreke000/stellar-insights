//! Generates the OpenAPI 3 spec and a Postman collection from the utoipa annotations.
//!
//! Usage: `cargo run --bin export_openapi -- [output_dir]` (default: `../docs/api`)

use std::path::PathBuf;

use serde_json::{json, Value};
use payraider_backend::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() -> anyhow::Result<()> {
    let out_dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "../docs/api".to_string()),
    );
    std::fs::create_dir_all(&out_dir)?;

    let spec: Value = serde_json::to_value(ApiDoc::openapi())?;
    std::fs::write(
        out_dir.join("openapi.json"),
        serde_json::to_string_pretty(&spec)?,
    )?;

    let collection = postman_collection(&spec);
    std::fs::write(
        out_dir.join("postman_collection.json"),
        serde_json::to_string_pretty(&collection)?,
    )?;

    println!(
        "Wrote {} and {}",
        out_dir.join("openapi.json").display(),
        out_dir.join("postman_collection.json").display()
    );
    Ok(())
}

/// Builds a Postman v2.1 collection grouped by OpenAPI tag.
fn postman_collection(spec: &Value) -> Value {
    let mut folders: std::collections::BTreeMap<String, Vec<Value>> = Default::default();

    if let Some(paths) = spec["paths"].as_object() {
        for (path, ops) in paths {
            let Some(ops) = ops.as_object() else { continue };
            for (method, op) in ops {
                let tag = op["tags"][0].as_str().unwrap_or("Other").to_string();
                let name = op["summary"]
                    .as_str()
                    .or_else(|| op["operationId"].as_str())
                    .unwrap_or(path)
                    .to_string();

                // {id} -> :id for Postman path variables
                let segments: Vec<String> = path
                    .trim_start_matches('/')
                    .split('/')
                    .map(|s| {
                        s.strip_prefix('{')
                            .and_then(|s| s.strip_suffix('}'))
                            .map_or_else(|| s.to_string(), |v| format!(":{v}"))
                    })
                    .collect();

                let query: Vec<Value> = op["parameters"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|p| p["in"] == "query")
                    .map(|p| json!({ "key": p["name"], "value": "", "disabled": true }))
                    .collect();

                let mut request = json!({
                    "method": method.to_uppercase(),
                    "header": [{ "key": "Content-Type", "value": "application/json" }],
                    "url": {
                        "raw": format!("{{{{baseUrl}}}}{path}"),
                        "host": ["{{baseUrl}}"],
                        "path": segments,
                        "query": query,
                    },
                    "description": op["description"],
                });

                if op.get("security").is_some() {
                    request["auth"] = json!({
                        "type": "bearer",
                        "bearer": [{ "key": "token", "value": "{{accessToken}}", "type": "string" }]
                    });
                }

                let body_schema = &op["requestBody"]["content"]["application/json"]["schema"];
                if !body_schema.is_null() {
                    let example = resolve_example(spec, body_schema);
                    request["body"] = json!({
                        "mode": "raw",
                        "raw": serde_json::to_string_pretty(&example).unwrap_or_default(),
                        "options": { "raw": { "language": "json" } }
                    });
                }

                folders
                    .entry(tag)
                    .or_default()
                    .push(json!({ "name": name, "request": request }));
            }
        }
    }

    json!({
        "info": {
            "name": spec["info"]["title"],
            "description": spec["info"]["description"],
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "variable": [
            { "key": "baseUrl", "value": "http://localhost:8080" },
            { "key": "accessToken", "value": "" }
        ],
        "item": folders
            .into_iter()
            .map(|(name, item)| json!({ "name": name, "item": item }))
            .collect::<Vec<_>>()
    })
}

/// Returns the schema's example (following a `$ref` into components) or an empty object.
fn resolve_example(spec: &Value, schema: &Value) -> Value {
    let schema = schema["$ref"]
        .as_str()
        .and_then(|r| r.strip_prefix("#/components/schemas/"))
        .map_or(schema, |name| &spec["components"]["schemas"][name]);
    schema
        .get("example")
        .cloned()
        .unwrap_or_else(|| json!({}))
}
