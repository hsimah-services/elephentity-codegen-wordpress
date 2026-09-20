use super::*;
use registration::{humanise, plural, render};

pub fn generate(schema: &Value) -> Vec<Value> {
    let mut files = vec![];
    let mut pages = serde_json::Map::new();
    for e in vals(&schema["entities"]) {
        if !admin_enabled(schema, e) {
            continue;
        }
        let name = s(&e["name"]);
        let label = e["config"]["label"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| humanise(name));
        let plural = e["config"]["pluralLabel"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| plural(&label));
        let mut fields = serde_json::Map::new();
        fields.insert("id".into(), json!("ID"));
        for field in vals(&e["fields"]) {
            let name = s(&field["name"]);
            fields.insert(
                name.into(),
                json!(humanise(&format!(
                    "{}{}",
                    name[..1].to_uppercase(),
                    &name[1..]
                ))),
            );
        }
        let fields = render(&Value::Object(fields), 1);
        for (kind, title, method) in [("list", &plural, "listing"), ("detail", &label, "detail")] {
            let body = format!("use Eleph\\WordPress\\Admin\\View;\n\nreturn static function (View $view, array $data): void {{\n    /** @var array<string, mixed> $data */\n    $view->{method}({}, {fields}, $data);\n}};\n", q(title));
            files.push(json!({"path":format!("admin/{name}/{kind}.php"),"body":body}));
        }
        pages.insert(name.into(), json!({"label":plural,"slug":format!("eleph-{}",s(&e["storage"]["table"])),"parent":e["config"]["adminMenu"],"list":format!("admin/{name}/list.php"),"detail":format!("admin/{name}/detail.php")}));
    }
    files.insert(0,json!({"path":"admin-pages.php","body":format!("return {};\n",render(&Value::Object(pages),0))}));
    files
}
