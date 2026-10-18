/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// RPI-RP2 BOOTSEL volume discovery and UF2 copying.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

// Third Party Imports

// Local Imports
use crate::error::{Error, Result};

/* === Constants === */

const VOLUME_PREFIX: &str = "RPI-RP2";

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

#[cfg(target_os = "linux")]
pub fn discover() -> Result<Vec<PathBuf>> {
    let mut volumes = mounted_volumes();
    if volumes.is_empty()
        && let Some(device) = bootsel_block_device()
    {
        volumes = mount_with_udisks(&device)?;
    }
    Ok(volumes)
}

#[cfg(target_os = "macos")]
pub fn discover() -> Result<Vec<PathBuf>> {
    Ok(bootsel_volumes_in(Path::new("/Volumes")))
}

#[cfg(windows)]
pub fn discover() -> Result<Vec<PathBuf>> {
    let mut volumes = Vec::new();
    for letter in b'A'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", char::from(letter)));
        if let Ok(contents) = fs::read_to_string(root.join("INFO_UF2.TXT"))
            && identifies_rp2040(&contents)
        {
            volumes.push(root);
        }
    }
    Ok(volumes)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub fn discover() -> Result<Vec<PathBuf>> {
    Err(Error::Flash("BOOTSEL volume discovery is not supported on this platform".into()))
}

pub fn exactly_one(mut volumes: Vec<PathBuf>) -> Result<PathBuf> {
    volumes.sort();
    match volumes.len() {
        1 => Ok(volumes.remove(0)),
        0 => Err(Error::Flash(
            "no mounted RPI-RP2 BOOTSEL volume found; hold BOOTSEL while \
             plugging the board in, or pass --device/--port to restart a \
             running node into its bootloader"
                .into(),
        )),
        count => Err(Error::Flash(format!("expected exactly one RPI-RP2 BOOTSEL volume, found {count}: {}", joined(&volumes)))),
    }
}

pub fn write_uf2(uf2: &Path, volume: &Path) -> Result<PathBuf> {
    let name = uf2.file_name().ok_or_else(|| Error::Flash(format!("{} has no file name", uf2.display())))?;
    let payload = fs::read(uf2)?;
    let destination = volume.join(name);
    let mut file = fs::File::create(&destination)?;
    file.write_all(&payload)?;
    let _ = file.sync_all();
    Ok(destination)
}

#[must_use]
pub fn bootsel_volumes_in(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut volumes: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| entry.file_name().to_str().is_some_and(is_bootsel_volume_name) && entry.path().is_dir())
        .map(|entry| entry.path())
        .collect();
    volumes.sort();
    volumes
}

/* === Private Functions === */

fn is_bootsel_volume_name(name: &str) -> bool {
    name.starts_with(VOLUME_PREFIX)
}

#[cfg(any(windows, test))]
fn identifies_rp2040(contents: &str) -> bool {
    contents.contains(VOLUME_PREFIX) || contents.contains("RP2040")
}

#[cfg(target_os = "linux")]
fn mounted_volumes() -> Vec<PathBuf> {
    [Path::new("/run/media"), Path::new("/media")].into_iter().flat_map(subdirectories).flat_map(|user| bootsel_volumes_in(&user)).collect()
}

#[cfg(target_os = "linux")]
fn subdirectories(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).collect()
}

#[cfg(target_os = "linux")]
fn bootsel_block_device() -> Option<PathBuf> {
    fs::canonicalize("/dev/disk/by-label/RPI-RP2").ok()
}

#[cfg(target_os = "linux")]
fn mount_with_udisks(device: &Path) -> Result<Vec<PathBuf>> {
    let output = std::process::Command::new("udisksctl").args(["mount", "-b"]).arg(device).output();
    let output = match output {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::Flash(format!(
                "BOOTSEL device {} is present but not mounted and udisksctl \
                 is unavailable; install udisks2 or mount the volume manually",
                device.display()
            )));
        }
        Err(error) => return Err(error.into()),
        Ok(output) => output,
    };
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(path) = parse_udisks_mount_path(&stdout) {
            return Ok(vec![path]);
        }
        return Ok(mounted_volumes());
    }
    // The desktop may have automounted concurrently; rescan before failing.
    let volumes = mounted_volumes();
    if volumes.is_empty() {
        return Err(Error::Flash(format!(
            "udisksctl could not mount BOOTSEL device {}: {}",
            device.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(volumes)
}

#[cfg(any(target_os = "linux", test))]
fn parse_udisks_mount_path(stdout: &str) -> Option<PathBuf> {
    let path = stdout.split(" at ").nth(1)?.trim().trim_end_matches('.');
    (!path.is_empty()).then(|| PathBuf::from(path))
}

fn joined(volumes: &[PathBuf]) -> String {
    volumes.iter().map(|volume| volume.display().to_string()).collect::<Vec<_>>().join(", ")
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_uf2_only() {
        let rp2040 = "UF2 Bootloader v3.0\nModel: Raspberry Pi RP2\nBoard-ID: RPI-RP2\n";
        let other = "UF2 Bootloader v3.0\nModel: Trinket M0\nBoard-ID: SAMD21GB18A-Trinket-v0\n";
        assert!(identifies_rp2040(rp2040));
        assert!(!identifies_rp2040(other));
    }

    #[test]
    fn exactly_accepts_volume() {
        let volume = PathBuf::from("/run/media/user/RPI-RP2");
        assert_eq!(exactly_one(vec![volume.clone()]).unwrap(), volume);
    }

    #[test]
    fn exactly_rejects_ambiguity() {
        let error = exactly_one(Vec::new()).unwrap_err();
        assert!(error.to_string().contains("no mounted RPI-RP2"));
        let error = exactly_one(vec![PathBuf::from("/run/media/user/RPI-RP2"), PathBuf::from("/run/media/user/RPI-RP21")]).unwrap_err();
        assert!(error.to_string().contains("found 2"));
        assert!(error.to_string().contains("RPI-RP21"));
    }

    #[test]
    fn udisks_mount_period() {
        assert_eq!(
            parse_udisks_mount_path("Mounted /dev/sda1 at /run/media/user/RPI-RP2.\n"),
            Some(PathBuf::from("/run/media/user/RPI-RP2"))
        );
        assert_eq!(parse_udisks_mount_path("Mounted /dev/sda1 at /media/user/RPI-RP2\n"), Some(PathBuf::from("/media/user/RPI-RP2")));
        assert_eq!(parse_udisks_mount_path("nonsense"), None);
    }
}
