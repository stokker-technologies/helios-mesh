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
import sys
import tomllib
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

# Third Party Imports

# Local Imports
from .. import temporary
from ..build import paths

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
    product = paths.output(ROOT, "build", options.profile)
    destination = paths.output(ROOT, "package", options.profile) / options.language
    if options.language == "python":
        if not any((product / "python").glob("*.whl")):
            raise ValueError("missing native wheels; run the matching Product Build first")
        paths.replace(ROOT, product / "python", destination)
        return

    executable = "helios-mesh.exe" if os.name == "nt" else "helios-mesh"
    with temporary.directory() as staging:
        shutil.copy2(product / "bin" / executable, staging / executable)
        if platform.system() == "Linux":
            firmware = staging / "firmware"
            firmware.mkdir()
            entries: dict[str, bytes] = {}
            entries["LICENSE"] = (ROOT / "LICENSE").read_bytes()
            for node in ("gateway", "rotator", "sidecar"):
                source = product / "firmware" / f"helios-mesh-node-{node}"
                image = firmware / f"helios-mesh-node-{node}.uf2"
                subprocess.run(
                    ["elf2uf2-rs", str(source), str(image)],
                    creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
                    stdin=sys.stdin,
                    stdout=sys.stdout,
                    stderr=sys.stderr,
                    cwd=ROOT,
                    check=True,
                )
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
            with ZipFile(firmware / "helios-mesh-firmware-feather-rp2040.zip", "w") as packaged:
                for name, content in sorted(entries.items()):
                    info = ZipInfo(name)
                    info.create_system = 3
                    info.external_attr = 0o100644 << 16
                    info.compress_type = ZIP_DEFLATED
                    packaged.writestr(info, content)
        paths.replace(ROOT, staging, destination, origin=staging)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
