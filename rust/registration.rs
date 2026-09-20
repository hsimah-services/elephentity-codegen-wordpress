use super::*;
use std::collections::BTreeMap;
fn configured<'a>(e: &'a Value, k: &str, default: &'a Value) -> &'a Value {
    e["config"].get(k).unwrap_or(default)
}
pub(crate) fn humanise(n: &str) -> String {
    n.chars()
        .enumerate()
        .map(|(i, c)| {
            if i > 0 && c.is_ascii_uppercase() {
                format!(" {c}")
            } else {
                c.to_string()
            }
        })
        .collect::<String>()
        .trim()
        .into()
}
pub(crate) fn plural(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    if ["s", "x", "z", "ch", "sh"]
        .iter()
        .any(|end| lower.ends_with(end))
    {
        format!("{s}es")
    } else if lower.ends_with('y')
        && lower.len() > 1
        && !"aeiou".contains(lower.as_bytes()[lower.len() - 2] as char)
    {
        format!("{}ies", &s[..s.len() - 1])
    } else {
        format!("{s}s")
    }
}
pub(crate) fn render(v: &Value, depth: usize) -> String {
    let entries: Vec<(String, &Value)> = match v {
        Value::Object(m) => m.iter().map(|(k, v)| (q(k), v)).collect(),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v))
            .collect(),
        Value::String(s) => return q(s),
        _ => return v.to_string(),
    };
    if entries.is_empty() {
        return "[]".into();
    }
    format!(
        "[\n{}\n{}]",
        entries
            .into_iter()
            .map(|(k, v)| format!(
                "{}{k} => {},",
                "    ".repeat(depth + 1),
                render(v, depth + 1)
            ))
            .collect::<Vec<_>>()
            .join("\n"),
        "    ".repeat(depth)
    )
}
pub fn generate(schema: &Value, tax: bool) -> String {
    let mut result = BTreeMap::new();
    let yes = json!(true);
    let no = json!(false);
    let post = json!("post");
    let null = Value::Null;
    for e in vals(&schema["entities"]) {
        if tax {
            if !taxonomy(e) {
                continue;
            }
        } else if !linked(schema, e) {
            continue;
        }
        let handle = s(&e["storage"]["handle"]);
        let singular = e["config"]["label"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| humanise(s(&e["name"])));
        let plural = e["config"]["pluralLabel"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| plural(&singular));
        let lower = plural.to_lowercase();
        let admin = b(configured(e, "showInAdmin", &yes));
        let rest = b(configured(e, "showInRest", &no));
        let description = e["description"].as_str().unwrap_or("");
        let args = if tax {
            let public = b(configured(e, "public", &yes));
            let hierarchical = b(configured(e, "hierarchical", &no));
            let mut objects = vec![];
            for source in vals(&schema["entities"]) {
                let handle = if linked(schema, source) {
                    source["storage"]["handle"].as_str()
                } else {
                    None
                };
                for edge in vals(&source["edges"]) {
                    if edge["to"] == e["name"] {
                        if let Some(h) = handle {
                            objects.push(h);
                        }
                    }
                }
            }
            json!({"labels":{"name":plural,"singular_name":singular,"search_items":format!("Search {plural}"),"all_items":format!("All {plural}"),"edit_item":format!("Edit {singular}"),"add_new_item":format!("Add New {singular}"),"new_item_name":format!("New {singular} Name")},"description":description,"public":public,"publicly_queryable":public,"hierarchical":hierarchical,"show_ui":admin,"show_admin_column":admin,"show_in_rest":rest,"object_type":objects})
        } else {
            let admin = admin && !admin_enabled(schema, e);
            let public = e["config"]["visibility"] == "public";
            let menu = configured(e, "adminMenu", &null)
                .as_str()
                .map(|s| json!(s))
                .unwrap_or(json!(admin));
            let menu = if admin { menu } else { json!(false) };
            let supports = list(&e["config"]["supports"])
                .into_iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            json!({"labels":{"name":plural,"singular_name":singular,"add_new_item":format!("Add New {singular}"),"edit_item":format!("Edit {singular}"),"new_item":format!("New {singular}"),"view_item":format!("View {singular}"),"search_items":format!("Search {plural}"),"not_found":format!("No {lower} found"),"not_found_in_trash":format!("No {lower} found in Trash")},"description":description,"public":public,"publicly_queryable":public,"exclude_from_search":!public,"show_ui":admin,"show_in_menu":menu,"show_in_rest":rest,"capability_type":configured(e,"capabilityType",&post),"supports":supports})
        };
        result.insert(handle.to_owned(), args);
    }
    let value = Value::Object(result.into_iter().collect());
    format!(
        "{}return {};\n",
        if tax {
            include_str!("taxonomy-header.txt")
        } else {
            include_str!("post-header.txt")
        },
        render(&value, 0)
    )
}
