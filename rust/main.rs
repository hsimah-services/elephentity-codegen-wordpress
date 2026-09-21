mod admin;
mod ir;
mod registration;
mod storage;
use serde_json::{json, Value};
use std::io::{self, Read};
type Result<T> = std::result::Result<T, String>;
fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap_or(false)
}
fn vals(v: &Value) -> Vec<&Value> {
    v.as_object()
        .map(|m| m.values().collect())
        .unwrap_or_default()
}
fn list(v: &Value) -> Vec<&Value> {
    v.as_array().map(|a| a.iter().collect()).unwrap_or_default()
}
fn q(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}
fn low(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_lowercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}
fn snake(s: &str) -> String {
    s.chars()
        .enumerate()
        .map(|(i, c)| {
            if i > 0 && c.is_ascii_uppercase() {
                format!("_{}", c.to_ascii_lowercase())
            } else {
                c.to_ascii_lowercase().to_string()
            }
        })
        .collect()
}
fn taxonomy(e: &Value) -> bool {
    b(&e["config"]["taxonomy"])
}
fn account(e: &Value) -> bool {
    b(&e["config"]["account"])
}
fn setting(schema: &Value, e: &Value, key: &str) -> bool {
    e["integrations"]["wordpress"][key]
        .as_bool()
        .or_else(|| schema["project"]["integrations"]["wordpress"][key].as_bool())
        .unwrap_or(key == "adminTemplates")
}
fn linked(schema: &Value, e: &Value) -> bool {
    !taxonomy(e)
        && !account(e)
        && e["storage"]["driver"] == "wordpress"
        && e["storage"]["handle"].as_str().is_some()
        && setting(schema, e, "linkPosts")
}
fn admin_enabled(schema: &Value, e: &Value) -> bool {
    !taxonomy(e)
        && !account(e)
        && setting(schema, e, "adminTemplates")
        && e["config"]["showInAdmin"].as_bool().unwrap_or(true)
}
fn response(files: Vec<Value>, errors: Vec<String>) -> Value {
    json!({"elephentity":1,"irVersion":"1.2","headerStyle":"php","extensions":["php"],"files":files,"errors":errors})
}
fn run(v: &Value) -> Result<Value> {
    if v["elephentity"].as_u64() != Some(1) {
        return Err("Protocol version mismatch: this builder speaks 1.".into());
    }
    if v["irVersion"] != "1.2" {
        return Err("IR version mismatch: this builder speaks 1.2.".into());
    }
    let kind = match v.get("request") {
        None | Some(Value::Null) => "generate",
        Some(Value::String(kind)) => kind.as_str(),
        Some(_) => {
            return Err(
                "Unknown request. This builder answers \"generate\" and \"describe\".".into(),
            )
        }
    };
    match kind {
        "describe" => {
            return Ok(
                json!({"elephentity":1,"irVersion":"1.2","provides":serde_json::from_str::<Value>(include_str!("provides.json")).map_err(|e|e.to_string())?}),
            )
        }
        "generate" => (),
        other => {
            return Err(format!(
                "Unknown request \"{other}\". This builder answers \"generate\" and \"describe\"."
            ))
        }
    }
    let schema = &v["schema"];
    if !schema.is_object() {
        return Err("The request carries no schema.".into());
    }
    if !schema["project"].is_object() {
        return Err("The schema is not readable: missing project.".into());
    }
    let normalized = ir::decode(schema)?;
    let schema = &normalized;
    if schema["project"]["driver"] != "wordpress" {
        return Ok(response(vec![],vec![format!("This project declares driver \"{}\", so the wordpress target has nothing to generate. Remove it from eleph.json, or change the driver in project.yml.",s(&schema["project"]["driver"]))]));
    }
    let storage = match storage::generate(schema) {
        Ok(body) => body,
        Err(e) => return Ok(response(vec![], vec![e])),
    };
    let mut files = vec![
        json!({"path":"storage-manifest.php","body":storage}),
        json!({"path":"post-types.php","body":registration::generate(schema,false)}),
        json!({"path":"taxonomies.php","body":registration::generate(schema,true)}),
    ];
    files.extend(admin::generate(schema));
    Ok(response(files, vec![]))
}
fn main() {
    let mut input = String::new();
    let r = io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())
        .and_then(|_| {
            if input.trim().is_empty() {
                Err("Expected a request on stdin.".into())
            } else {
                serde_json::from_str::<Value>(&input)
                    .map_err(|e| format!("Expected one JSON object on stdin: {e}"))
            }
        })
        .and_then(|v| run(&v));
    match r {
        Ok(v) => print!("{v}"),
        Err(e) => {
            eprintln!("eleph-gen-wordpress: {e}");
            std::process::exit(1);
        }
    }
}
