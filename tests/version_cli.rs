use assert_cmd::Command;

#[test]
fn version_flag_prints_binary_name_and_package_version() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["--version"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        format!("gitee {}", env!("CARGO_PKG_VERSION"))
    );
}
