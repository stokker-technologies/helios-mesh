/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Derive The Firmware Version From Cargo's Workspace Package Version.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{env, fs, path::PathBuf};

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

fn main() {
    let major = component("CARGO_PKG_VERSION_MAJOR");
    let minor = component("CARGO_PKG_VERSION_MINOR");
    let patch = component("CARGO_PKG_VERSION_PATCH");
    let source =
        format!("pub const FIRMWARE_VERSION: FirmwareVersion = FirmwareVersion {{ major: {major}, minor: {minor}, patch: {patch} }};\n");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
    fs::write(output.join("firmware-version.rs"), source).expect("write firmware version");
    println!("cargo:rerun-if-changed=Cargo.toml");
}

fn component(name: &str) -> u8 {
    let value = env::var(name).expect("Cargo sets package version components");
    value.parse().expect("firmware version components must fit in one byte")
}

/* === Tests === */
