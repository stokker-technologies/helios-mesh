# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Assemble CLI, Firmware And Wheel Packages From The Selected Build."""

# === Imports === #

# Standard Library Imports
import argparse
import os
import platform
import shutil
import subprocess
import tomllib
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

# Third Party Imports

# Local Imports
from ..test import archive

# Type Checking Imports

# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[5]

# === Types === #

# === Classes === #

# === Public Functions === #


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=("dev", "release"), default="dev")
    parser.add_argument("--language", choices=("rust", "python"), required=True)
    options = parser.parse_args()
    wheels = sorted((ROOT / "target/wheels").glob("*.whl"))
    if not wheels:
        raise archive.policy.TestError("missing native wheels; run the build task first")
    required = [path.relative_to(ROOT).as_posix() for path in wheels]
    if options.language == "rust":
        required.append("target/fixtures/bin/helios-mesh{exe}")
    if options.language == "rust" and platform.system() == "Linux":
        required.extend(f"target/fixtures/firmware/helios-mesh-node-{node}" for node in ("gateway", "rotator", "sidecar"))
    archive.fixtures(ROOT, options.profile, required)

    # Keep Each Native Wheel In Its Platform's Package Artifact.
    destination = ROOT / "target/package/python"
    destination.mkdir(parents=True, exist_ok=True)
    expected = {wheel.name for wheel in wheels}
    for old in destination.glob("*.whl"):
        if old.name not in expected:
            old.unlink(missing_ok=True)
    for wheel in wheels:
        temporary = destination / f".{wheel.name}.{os.getpid()}"
        try:
            shutil.copy2(wheel, temporary)
            temporary.replace(destination / wheel.name)
        finally:
            temporary.unlink(missing_ok=True)
    if options.language == "python":
        return

    executable = "helios-mesh.exe" if os.name == "nt" else "helios-mesh"
    source = ROOT / "target/fixtures/bin" / executable
    native = ROOT / "target/package/native"
    native.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, native / executable)

    if platform.system() == "Linux":
        firmware = ROOT / "target/package/firmware"
        firmware.mkdir(parents=True, exist_ok=True)
        entries: dict[str, bytes] = {}
        entries["LICENSE-APACHE"] = (ROOT / "LICENSE-APACHE").read_bytes()
        for node in ("gateway", "rotator", "sidecar"):
            source = ROOT / "target/fixtures/firmware" / f"helios-mesh-node-{node}"
            image = firmware / f"helios-mesh-node-{node}.uf2"
            subprocess.run(["elf2uf2-rs", str(source), str(image)], cwd=ROOT, check=True)
            entries[image.name] = image.read_bytes()
            entries[f"debug/{source.name}.elf"] = source.read_bytes()
        entries["README.txt"] = (
            "Helios Mesh Firmware - Feather RP2040\n\n"
            f"Version: {tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['package']['version']}\n"
            "Wire Protocol: 14\n"
            f"Build Profile: {options.profile}\n"
            "Standard Hardware: Feather RP2040 with RFM95 radio and the declared FeatherWing.\n"
            "Gateway: TFT FeatherWing V2. Rotator: Motor FeatherWing. Sidecar: OLED FeatherWing.\n\n"
            "Development hardware-unknown builds are USB-only diagnostics using the same filenames.\n"
            "Confirm your build options before selecting an image.\n\n"
            "Extract this archive, then select the UF2 matching the board's attached hardware.\n"
            "Hold BOOTSEL while connecting USB, then copy that UF2 to the RPI-RP2 volume.\n"
            "Flash one board at a time. The debug directory holds the matching ELF images.\n"
            "The firmware runs on RP2040; the Linux label identifies its build host.\n\n"
            "Use host software and firmware from the same revision and wire protocol version.\n"
            "Attach suitable antennas before powering radio firmware. Read the board safety guide:\n"
            "https://github.com/stokker-technologies/helios-mesh/blob/dev/"
            "crates/firmware/helios-mesh-board-feather-rp2040/README.md\n"
        ).encode()
        bundle = firmware / "helios-mesh-firmware-feather-rp2040.zip"
        temporary = bundle.with_name(f".{bundle.name}.{os.getpid()}")
        try:
            with ZipFile(temporary, "w") as packaged:
                for name, content in sorted(entries.items()):
                    info = ZipInfo(name)
                    info.create_system = 3
                    info.external_attr = 0o100644 << 16
                    info.compress_type = ZIP_DEFLATED
                    packaged.writestr(info, content)
            temporary.replace(bundle)
        finally:
            temporary.unlink(missing_ok=True)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
