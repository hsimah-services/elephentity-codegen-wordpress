use super::*;
use std::collections::BTreeMap;
struct Table {
    name: String,
    columns: Vec<(String, String)>,
    indexes: Vec<(String, String)>,
    primary: String,
}
struct Placement {
    entity: String,
    edge: String,
    target: String,
    relation: &'static str,
    table: String,
    local: String,
    remote: Option<String>,
    target_table: String,
}
fn column(name: &str, ty: &str, nullable: bool, auto: bool) -> String {
    format!(
        "new Column({}, {}, {nullable}, {auto}, null)",
        q(name),
        q(ty)
    )
}
fn index(table: &str, key: &str, columns: &[String], unique: bool) -> (String, String) {
    let name = format!("{table}_{key}_{}", if unique { "uniq" } else { "idx" })
        .chars()
        .take(64)
        .collect::<String>();
    let expr = format!(
        "new Index({}, [{}], {unique})",
        q(&name),
        columns.iter().map(|s| q(s)).collect::<Vec<_>>().join(", ")
    );
    (name, expr)
}
fn size(values: &Value) -> String {
    let values = list(values);
    format!(
        "VARCHAR({})",
        if values.is_empty() {
            64
        } else {
            values.iter().map(|v| s(v).len()).max().unwrap_or(1).max(1)
        }
    )
}
fn column_type(schema: &Value, f: &Value) -> Result<String> {
    let mut p = s(&f["type"]["primitive"]);
    if p.is_empty() {
        let n = s(&f["type"]["declaredType"]);
        let t = &schema["types"][n];
        if t.is_null() {
            return Err(format!("Field {} references declared type \"{n}\", which the schema compiler should already have proved exists.",s(&f["name"])));
        }
        if !t["values"].is_null() {
            return Ok(size(&t["values"]));
        }
        p = s(&t["primitive"]);
    }
    Ok(match p {
        "string" => format!("VARCHAR({})", f["maxLength"].as_u64().unwrap_or(255)),
        "text" | "json" => "LONGTEXT".into(),
        "int" => "BIGINT".into(),
        "float" => "DOUBLE".into(),
        "bool" => "TINYINT(1)".into(),
        "datetime" => "DATETIME".into(),
        "id" => "BIGINT UNSIGNED".into(),
        "enum" => {
            let en = &f["enum"];
            if en.is_null() {
                "VARCHAR(64)".into()
            } else {
                size(if en["inlineValues"].is_null() {
                    &schema["types"][s(&en["declaredType"])]["values"]
                } else {
                    &en["inlineValues"]
                })
            }
        }
        _ => return Err(format!("Unknown primitive {p}.")),
    })
}
fn pairs(entries: &[(String, String)], indent: usize) -> String {
    entries
        .iter()
        .map(|(k, v)| format!("{}{} => {v},", " ".repeat(indent), q(k)))
        .collect::<Vec<_>>()
        .join("\n")
}
fn table_expr(t: &Table) -> String {
    format!("new TableSchema(\n            {},\n            [\n{}\n            ],\n            [\n{}\n            ],\n            {},\n        )",q(&t.name),pairs(&t.columns,16),pairs(&t.indexes,16),q(&t.primary))
}
pub fn generate(schema: &Value) -> Result<String> {
    let entities = vals(&schema["entities"]);
    let mut tables: BTreeMap<String, Table> = BTreeMap::new();
    let mut placements = vec![];
    let mut tax_places = vec![];
    for e in &entities {
        let en = s(&e["name"]);
        let table = s(&e["storage"]["table"]);
        if !taxonomy(e) && !account(e) {
            if setting(schema, e, "linkPosts") && e["storage"]["handle"].as_str().is_none() {
                return Err(format!(
                    "Entity {en} enables WordPress post linking but has no storage.handle."
                ));
            }
            let mut t = Table {
                name: table.into(),
                columns: vec![("id".into(), column("id", "BIGINT UNSIGNED", false, true))],
                indexes: vec![],
                primary: "id".into(),
            };
            if linked(schema, e) {
                t.columns.push((
                    "wp_post_id".into(),
                    column("wp_post_id", "BIGINT UNSIGNED", true, false),
                ));
                t.indexes
                    .push(index(table, "wp_post_id", &["wp_post_id".into()], true));
            }
            for f in vals(&e["fields"]) {
                let n = snake(s(&f["name"]));
                if t.columns.iter().any(|(k, _)| k == &n) {
                    return Err(format!(
                        "Fields {en}.{} and another map to the same column \"{n}\".",
                        s(&f["name"])
                    ));
                }
                t.columns.push((
                    n.clone(),
                    column(&n, &column_type(schema, f)?, b(&f["nullable"]), false),
                ));
                if b(&f["unique"]) || b(&f["indexed"]) {
                    t.indexes
                        .push(index(table, &n, std::slice::from_ref(&n), b(&f["unique"])));
                }
            }
            tables.insert(table.into(), t);
        }
        for edge in vals(&e["edges"]) {
            let target = &schema["entities"][s(&edge["to"])];
            if target.is_null() {
                continue;
            }
            let to = s(&target["name"]);
            let name = s(&edge["name"]);
            let one = edge["cardinality"] == "one";
            let unique = b(&edge["inverse"]["unique"]);
            let relation = if one {
                if unique {
                    "OneToOne"
                } else {
                    "ManyToOne"
                }
            } else if unique {
                "OneToMany"
            } else {
                "ManyToMany"
            };
            if relation == "ManyToMany" && taxonomy(target) {
                tax_places.push((format!("{en}.{name}"),format!("new TaxonomyPlacement(\n            {},\n            {},\n            {},\n            {},\n        )",q(en),q(name),q(to),q(s(&target["storage"]["handle"])))));
                continue;
            }
            let tt = s(&target["storage"]["table"]);
            let (pt, local, remote) = if relation == "ManyToMany" {
                (
                    format!("{table}_{}", snake(name)),
                    format!("{}_id", snake(&low(en))),
                    Some(format!(
                        "{}_id",
                        snake(&low(if en == to { name } else { to }))
                    )),
                )
            } else if one {
                (table.into(), format!("{}_id", snake(name)), None)
            } else {
                let inverse = if b(&edge["inverse"]["derived"]) || edge["inverse"].is_null() {
                    low(en)
                } else {
                    s(&edge["inverse"]["name"]).into()
                };
                (tt.into(), format!("{}_id", snake(&inverse)), None)
            };
            placements.push(Placement {
                entity: en.into(),
                edge: name.into(),
                target: to.into(),
                relation,
                table: pt,
                local,
                remote,
                target_table: tt.into(),
            });
        }
    }
    for p in &placements {
        if let Some(right) = &p.remote {
            let left = &p.local;
            let mut columns = vec![(left.clone(), column(left, "BIGINT UNSIGNED", false, false))];
            if left == right {
                columns[0].1 = column(right, "BIGINT UNSIGNED", false, false);
            } else {
                columns.push((
                    right.clone(),
                    column(right, "BIGINT UNSIGNED", false, false),
                ));
            }
            tables.insert(
                p.table.clone(),
                Table {
                    name: p.table.clone(),
                    columns,
                    indexes: vec![
                        index(&p.table, "pair", &[left.clone(), right.clone()], true),
                        index(&p.table, right, std::slice::from_ref(right), false),
                    ],
                    primary: String::new(),
                },
            );
        } else if let Some(t) = tables.get_mut(&p.table) {
            if t.columns.iter().any(|(n, _)| n == &p.local) {
                return Err(format!(
                    "Edge {}.{} needs column \"{}\" on {}, but a field already claims it.",
                    p.entity, p.edge, p.local, p.table
                ));
            }
            t.columns.push((
                p.local.clone(),
                column(&p.local, "BIGINT UNSIGNED", true, false),
            ));
            t.indexes.push(index(
                &p.table,
                &p.local,
                std::slice::from_ref(&p.local),
                p.relation == "OneToOne",
            ));
        }
    }
    let mut entity_tables = BTreeMap::new();
    let mut columns = BTreeMap::new();
    let mut taxonomies = BTreeMap::new();
    let mut accounts = BTreeMap::new();
    let mut posts = BTreeMap::new();
    let claimed = entities
        .iter()
        .map(|e| s(&e["storage"]["table"]))
        .collect::<Vec<_>>();
    for e in &entities {
        let en = s(&e["name"]);
        if linked(schema, e) {
            posts.insert(en.to_owned(), q(s(&e["storage"]["handle"])));
        }
        if let Some(t) = tables.get(s(&e["storage"]["table"])) {
            entity_tables.insert(en.to_owned(), table_expr(t));
        }
        let fields = vals(&e["fields"]);
        if !fields.is_empty() {
            columns.insert(
                en.to_owned(),
                format!(
                    "[{}]",
                    fields
                        .iter()
                        .map(|f| format!("{} => {}", q(s(&f["name"])), q(&snake(s(&f["name"])))))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
        if taxonomy(e) {
            taxonomies.insert(en.into(), q(s(&e["storage"]["handle"])));
        }
        if account(e) {
            let mut created = "null".into();
            let mut modified = "null".into();
            for f in &fields {
                if f["managed"] == "created" {
                    created = q(s(&f["name"]));
                }
                if f["managed"] == "modified" {
                    modified = q(s(&f["name"]));
                }
            }
            accounts.insert(en.into(),format!("new AccountFields(\n            {},\n            {created},\n            {modified},\n        )",q(en)));
        }
    }
    let join_tables = tables
        .iter()
        .filter(|(n, _)| !claimed.contains(&n.as_str()))
        .map(|(n, t)| (n.clone(), table_expr(t)))
        .collect::<Vec<_>>();
    let placement_exprs=placements.iter().map(|p|(format!("{}.{}",p.entity,p.edge),format!("new EdgePlacement(\n            {},\n            {},\n            {},\n            RelationKind::{},\n            {},\n            {},\n            {},\n            {},\n        )",q(&p.entity),q(&p.edge),q(&p.target),p.relation,q(&p.table),q(&p.local),p.remote.as_ref().map(|s|q(s)).unwrap_or("null".into()),q(&p.target_table)))).collect::<Vec<_>>();
    let mut body="namespace Eleph\\WordPress\\Manifest;\n\nuse Eleph\\WordPress\\Account\\AccountFields;\nuse Eleph\\WordPress\\Sql\\Column;\nuse Eleph\\WordPress\\Sql\\EdgePlacement;\nuse Eleph\\WordPress\\Sql\\Index;\nuse Eleph\\WordPress\\Sql\\TableSchema;\nuse Eleph\\WordPress\\Taxonomy\\TaxonomyPlacement;\nuse Eleph\\Runtime\\Storage\\RelationKind;\n\nreturn new StorageManifest(\n".to_owned();
    body = body.replace(
        "return new StorageManifest(\n",
        &format!(
            "{}return new StorageManifest(\n",
            include_str!("storage-header.txt")
        ),
    );
    for (n, entries) in [
        ("tables", entity_tables.into_iter().collect()),
        ("placements", placement_exprs),
        ("columns", columns.into_iter().collect()),
        ("joinTables", join_tables),
        ("taxonomies", taxonomies.into_iter().collect()),
        ("taxonomyPlacements", tax_places),
        ("accounts", accounts.into_iter().collect()),
        ("posts", posts.into_iter().collect()),
    ] {
        body.push_str(&format!("    {n}: [\n{}\n    ],\n", pairs(&entries, 8)));
    }
    body.push_str(");\n");
    Ok(body)
}
