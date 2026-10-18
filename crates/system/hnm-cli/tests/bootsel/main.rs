/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Verify BOOTSEL Discovery And UF2 Copying Without Accessing Physical Devices.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{fs, path::PathBuf};

// Third Party Imports

// Local Imports
use hnm_cli::bootsel::{bootsel_volumes_in, write_uf2};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

fn workspace() -> tempfile::TempDir {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/tmp");
    fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

/* === Tests === */

#[test]
fn discover_volumes() {
    let scratch = workspace();
    let root = scratch.path();
    fs::create_dir_all(root.join("RPI-RP2")).unwrap();
    fs::create_dir_all(root.join("RPI-RP21")).unwrap();
    fs::create_dir_all(root.join("BACKUP")).unwrap();
    fs::write(root.join("RPI-RP2X"), b"a file, not a mount").unwrap();

    let volumes = bootsel_volumes_in(root);
    assert_eq!(volumes, vec![root.join("RPI-RP2"), root.join("RPI-RP21")]);
}

#[test]
fn copy_firmware() {
    let scratch = workspace();
    let root = scratch.path();
    let volume = root.join("RPI-RP2");
    fs::create_dir_all(&volume).unwrap();
    let source = root.join("hnm-node-gateway.uf2");
    fs::write(&source, b"UF2\n").unwrap();

    let destination = write_uf2(&source, &volume).unwrap();
    assert_eq!(destination, volume.join("hnm-node-gateway.uf2"));
    assert_eq!(fs::read(destination).unwrap(), b"UF2\n");
}
