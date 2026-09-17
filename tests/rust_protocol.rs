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
        json!({"elephentity":2,"irVersion":"1.1","request":"describe"}),
        json!({"elephentity":1,"irVersion":"1.1","target":"test","outputDirectory":"out","schema":{"project":{}}}),
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
        "elephentity":1,"irVersion":"1.1","target":"test","outputDirectory":"out",
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
