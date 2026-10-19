/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Implement main for system helios_mesh cli tests acceptance.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{fs, path::PathBuf, process::Command};

// Third Party Imports

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

fn executable() -> PathBuf {
    if let Some(path) = std::env::var_os("NEXTEST_BIN_EXE_helios-mesh") {
        return PathBuf::from(path);
    }
    match option_env!("CARGO_BIN_EXE_helios-mesh") {
        Some(path) => PathBuf::from(path),
        None => panic!("Build must supply the CLI executable"),
    }
}

/* === Tests === */

#[test]
fn help_version_loading() {
    for option in ["--help", "--version"] {
        let output = Command::new(executable()).args(["--env-file", "/missing/helios_mesh.env", option]).output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stderr, []);
    }
}

#[test]
fn invalid_environment_access() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join(".env"), "INVALID='unterminated").unwrap();

    let output = Command::new(executable()).current_dir(directory.path()).arg("status").output().unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid environment configuration"));
}

#[test]
fn missing_explicit_error() {
    let output = Command::new(executable()).args(["--env-file", "/missing/helios_mesh.env", "status"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}
