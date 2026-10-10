use serde_json::{json, Value};
use std::{
    io::Write,
    process::{Command, Stdio},
};
fn invoke(request: &Value) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_eleph-gen-wordpress"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let payload = request.to_string();
    let writer = std::thread::spawn(move || stdin.write_all(payload.as_bytes()).unwrap());
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    output
}
#[test]
fn golden_responses() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden");
    for entry in std::fs::read_dir(root).unwrap() {
        let dir = entry.unwrap().path();
        let request: Value =
            serde_json::from_slice(&std::fs::read(dir.join("request.json")).unwrap()).unwrap();
        let expected: Value =
            serde_json::from_slice(&std::fs::read(dir.join("response.json")).unwrap()).unwrap();
        let output = invoke(&request);
        assert!(
            output.status.success(),
            "{}: {}",
            dir.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(expected, actual, "{}", dir.display());
    }
}
#[test]
fn rejects_unsupported_versions_and_malformed_schema() {
    for request in [
        json!({"elephentity":1,"irVersion":"99","request":"describe"}),
        json!({"elephentity":2,"irVersion":"1.2","request":"describe"}),
        json!({"elephentity":1,"irVersion":"1.2","target":"test","outputDirectory":"out","schema":{"project":{}}}),
    ] {
        let output = invoke(&request);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn nullable_ir_members_still_require_their_keys() {
    let mut request = json!({
        "elephentity":1,"irVersion":"1.2","target":"test","outputDirectory":"out",
        "config":{"namespace":"Example","typeNamespace":"ExampleType"},
        "schema":{"project":{"name":"Test","driver":"wordpress","sourceFile":"project.yml"},
        "entities":{"Item":{"name":"Item","storage":{"driver":"wordpress","table":"item"},"sourceFile":"item.yml",
        "fields":{"name":{"name":"name","type":{"primitive":"string","declaredType":null},"origin":{"pattern":null,"file":"item.yml"}}}}}}
    });
    request["schema"]["entities"]["Item"]["fields"]["name"]["type"]
        .as_object_mut()
        .unwrap()
        .remove("declaredType");
    let output = invoke(&request);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

fn settings_request() -> Value {
    json!({"elephentity":1,"irVersion":"1.2","schema":{
        "project":{"name":"Demo","driver":"wordpress","sourceFile":"project.yml"},
        "entities":{"Item":{"name":"Item","sourceFile":"item.yml","storage":{"driver":"wordpress","table":"item","handle":"item"}},
        "Owner":{"name":"Owner","sourceFile":"owner.yml","storage":{"driver":"wordpress","table":"owner","handle":"owner"},"config":{"account":true}}}
    }})
}
fn generated(request: &Value) -> Value {
    let output = invoke(request);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn body(response: &Value, path: &str) -> String {
    response["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == path)
        .unwrap()["body"]
        .as_str()
        .unwrap()
        .into()
}
#[test]
fn admin_defaults_on_and_posts_default_off() {
    let result = generated(&settings_request());
    assert_eq!(result["errors"], json!([]));
    assert!(body(&result, "admin-pages.php").contains("'Item'"));
    assert!(!body(&result, "admin-pages.php").contains("'Owner'"));
    assert!(body(&result, "admin/Item/list.php").contains("$view->listing"));
    assert!(body(&result, "admin/Item/detail.php").contains("$view->detail"));
    assert!(!body(&result, "storage-manifest.php").contains("wp_post_id"));
    assert!(!body(&result, "post-types.php").contains("'item'"));
}
#[test]
fn posts_and_templates_are_independent_and_entity_settings_override_project() {
    let mut request = settings_request();
    request["schema"]["project"]["integrations"] =
        json!({"wordpress":{"linkPosts":true,"adminTemplates":false}});
    let result = generated(&request);
    assert!(body(&result, "storage-manifest.php").contains("'wp_post_id' => new Column"));
    assert!(body(&result, "storage-manifest.php").contains("'Item' => 'item'"));
    assert!(body(&result, "post-types.php").contains("'show_ui' => true"));
    assert_eq!(body(&result, "admin-pages.php"), "return [];\n");
    request["schema"]["entities"]["Item"]["integrations"] =
        json!({"wordpress":{"adminTemplates":true,"linkPosts":null}});
    let result = generated(&request);
    assert!(body(&result, "post-types.php").contains("'show_ui' => false"));
    assert!(body(&result, "post-types.php").contains("'show_in_menu' => false"));
    request["schema"]["entities"]["Item"]["integrations"]["wordpress"]["linkPosts"] = json!(false);
    let result = generated(&request);
    assert!(!body(&result, "storage-manifest.php").contains("wp_post_id"));
    assert!(!body(&result, "post-types.php").contains("'item'"));
    assert!(body(&result, "admin-pages.php").contains("'Item'"));
}
#[test]
fn linking_rejects_collision_with_an_entity_field() {
    let mut request = settings_request();
    request["schema"]["project"]["integrations"] = json!({"wordpress":{"linkPosts":true}});
    request["schema"]["entities"]["Item"]["fields"] = json!({"wpPostId":{"name":"wpPostId","type":{"primitive":"id","declaredType":null},"origin":{"pattern":null,"file":"item.yml"}}});
    let result = generated(&request);
    assert!(result["files"].as_array().unwrap().is_empty());
    assert!(result["errors"][0].as_str().unwrap().contains("wp_post_id"));
}

#[test]
fn explicitly_linked_entities_require_a_post_type_handle() {
    let mut request = settings_request();
    request["schema"]["entities"]["Item"]["storage"]["handle"] = Value::Null;
    request["schema"]["entities"]["Item"]["integrations"] = json!({"wordpress":{"linkPosts":true}});
    let result = generated(&request);
    assert_eq!(result["files"], json!([]));
    assert!(result["errors"][0]
        .as_str()
        .unwrap()
        .contains("storage.handle"));
}

#[test]
fn join_tables_cannot_replace_entities_or_other_edges() {
    let original: Value =
        serde_json::from_str(include_str!("fixtures/golden/relations/request.json")).unwrap();
    for other_edge in [false, true] {
        let mut request = original.clone();
        request["schema"]["entities"]["Post"]["storage"]["table"] = json!("post");
        request["schema"]["entities"]["Post"]["edges"]["tags"]["name"] = json!("fooBar");
        let second_owner = if other_edge {
            let mut edge = request["schema"]["entities"]["Post"]["edges"]["tags"].clone();
            edge["name"] = json!("foo_bar");
            request["schema"]["entities"]["Post"]["edges"]["foo_bar"] = edge;
            "Post.foo_bar"
        } else {
            request["schema"]["entities"]["Author"]["storage"]["table"] = json!("post_foo_bar");
            "Author"
        };
        let output = invoke(&request);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(response["files"], json!([]));
        let errors = response["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1);
        let error = errors[0].as_str().unwrap();
        for owner in ["Post.fooBar", second_owner, "post_foo_bar"] {
            assert!(error.contains(owner), "{error}");
        }
    }
}

#[test]
fn taxonomy_edges_get_distinct_domain_join_tables() {
    let mut request: Value =
        serde_json::from_str(include_str!("fixtures/golden/clog/request.json")).unwrap();
    let mut edge = request["schema"]["entities"]["Location"]["edges"]["sites"].clone();
    edge["name"] = json!("otherSites");
    request["schema"]["entities"]["Location"]["edges"]["otherSites"] = edge;
    let response = generated(&request);
    assert_eq!(response["errors"], json!([]));
    let body = response["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == "storage-manifest.php")
        .unwrap()["body"]
        .as_str()
        .unwrap();
    for table in [
        "clog_inventory_site",
        "clog_location_sites",
        "clog_location_other_sites",
    ] {
        assert!(
            body.contains(&format!("'{table}' => new TableSchema")),
            "{table}"
        );
    }
    assert!(!body.contains("new TaxonomyPlacement("));
}
