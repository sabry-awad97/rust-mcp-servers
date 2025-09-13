use assert_cmd::Command;

/// Test CLI help output
#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("mcp_server_filesystem").unwrap();
    let assert = cmd.arg("--help").assert();

    assert.success();
}

/// Test CLI version output
#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("mcp_server_filesystem").unwrap();
    let assert = cmd.arg("--version").assert();

    assert.success();
}

/// Test with directory argument
#[test]
fn test_with_directory() {
    let mut cmd = Command::cargo_bin("mcp_server_filesystem").unwrap();
    let assert = cmd.arg(".").assert();

    assert.success();
}

/// Test with multiple directories
#[test]
fn test_with_multiple_directories() {
    let mut cmd = Command::cargo_bin("mcp_server_filesystem").unwrap();
    let assert = cmd.args([".", "src"]).assert();

    assert.success();
}
