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
import hashlib
import os
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tomllib
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

# Third Party Imports
# Local Imports
from .. import policy, temporary
from ..build import paths
from ..process import spawn_flags

# Type Checking Imports

# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[5]

# === Types === #

# === Classes === #

# === Public Functions === #


def wheel_inventory(names: list[str], targets: list[str], version: str | None = None) -> dict[str, str]:
    """Require Exactly One Stable-ABI Wheel Per Target At One Shared Version."""
    wheels = [name for name in names if name.endswith(".whl")]
    if len(wheels) != len(targets):
        raise ValueError("run the matching Product Builds to produce exactly one wheel per target")
    if version is None:
        version = wheels[0].split("-")[1] if wheels and "-" in wheels[0] else ""
    platform_tags = {
        "linux-x86_64": r"manylinux_2_[0-9]+_x86_64(?:\.manylinux2014_x86_64)?",
        "windows-x86_64": "win_amd64",
    }
    result = {}
    for selected in targets:
        pattern = rf"helios_mesh-{re.escape(version)}-cp310-abi3-{platform_tags[selected]}\.whl"
        matches = [name for name in wheels if re.fullmatch(pattern, name)]
        if len(matches) != 1:
            raise ValueError(f"rebuild the Python wheel for {selected} with CPython 3.10+ ABI3 and a portable platform tag")
        result[selected] = matches[0]
    return result


def wheel(directory: Path, selected: str, version: str) -> Path:
    """Resolve The Matching Build's Wheel Using The Shared Inventory Contract."""
    inventory = wheel_inventory([path.name for path in directory.glob("*.whl")], [selected], version)
    source = directory / inventory[selected]
    if source.is_symlink():
        raise ValueError(f"rebuild the Python wheel as a regular file: {source}")
    return source


def definitions(directory: Path, profile: str, target: str) -> list[dict]:
    """Resolve Declared Products Without Looking At Native Compiler Directories."""
    config = policy.config(directory)
    version = config.get("version", {}).get("value")
    result = []
    for item in config.get("package", {}).get("executables", []):
        triple = item["targets"].get(target)
        if triple is None:
            continue
        if not version or not re.fullmatch(r"[a-zA-Z0-9_.-]+", version):
            raise policy.ToolError("executable packages require version.value")
        extension = ".exe" if "-windows-" in triple else ""
        archive = ".zip" if extension else ".tar.gz"
        name = f"{item['name']}-{version}-{triple}{archive}"
        result.append({**item, "language": "rust", "output": name, "triple": triple, "binary": item["command"] + extension})
    for item in config.get("package", {}).get("bundles", []):
        if profile in item["profiles"] and target in item["targets"]:
            fields = [item["name"], *(entry[key] for entry in item["inputs"] for key in ("path", "destination"))]
            if any("{version}" in field for field in fields) and (
                not isinstance(version, str) or re.fullmatch(r"[A-Za-z0-9_.-]+", version) is None or version in {".", ".."}
            ):
                raise policy.ToolError("versioned bundles require a safe version.value")
            result.append({**item, "output": item["name"].format(version=version, target=target)})
    names = [item["output"].casefold() for item in result]
    if len(names) != len(set(names)):
        raise policy.ToolError("package output names collide")
    for item in result:
        name = item["output"]
        if re.fullmatch(r"[A-Za-z0-9_.-]+", name) is None or name in {".", "..", "SHA256SUMS", "release.json", "platform.json"}:
            raise policy.ToolError("unsafe or reserved package output name")
    return result


def packaged(directory: Path, profile: str, target: str) -> list[dict]:
    """Only Explicitly Shipped Tools Enter The Release; Bundles Need release.assets."""
    root = paths.output(directory, "package", profile, target)
    result = [
        {"target": target, "path": (root / item["language"] / item["output"]).relative_to(directory).as_posix(), "name": item["output"]}
        for item in definitions(directory, profile, target)
        if "triple" in item and item["release"]
    ]
    return result


def binary_bytes(source: Path, name: str) -> bytes:
    """Read Only The Declared Root Binary, Without Extracting Archive Paths."""
    if source.name.endswith(".zip"):
        with ZipFile(source) as archive:
            matches = [entry for entry in archive.infolist() if entry.filename == name]
            if len(matches) != 1 or matches[0].is_dir():
                raise policy.ToolError("tool archive must contain one declared binary")
            return archive.read(matches[0])
    with tarfile.open(source, "r:gz") as archive:
        matches = [entry for entry in archive.getmembers() if entry.name == name]
        if len(matches) != 1 or not matches[0].isfile() or not matches[0].mode & 0o111:
            raise policy.ToolError("tool archive must contain one executable regular binary")
        stream = archive.extractfile(matches[0])
        if stream is None:
            raise policy.ToolError("tool archive binary has no file content")
        try:
            return stream.read()
        finally:
            stream.close()


def verify(directory: Path, profile: str, target: str, root: Path | None = None) -> None:
    """Execute Packaged Native Bytes In An Isolated Installation Directory."""
    for item in definitions(directory, profile, target):
        # Staged release downloads hold only the executables that ship.
        if "triple" not in item or root is not None and not item["release"]:
            continue
        source = (root / item["output"]) if root else paths.output(directory, "package", profile, target) / "rust" / item["output"]
        if not source.is_file() or source.is_symlink():
            raise policy.ToolError("missing packaged executable")
        with temporary.directory() as install:
            executable = install / item["binary"]
            executable.write_bytes(binary_bytes(source, item["binary"]))
            executable.chmod(0o755)
            subprocess.run(
                [str(executable), *item["verify"]],
                stdin=sys.stdin,
                stdout=sys.stdout,
                stderr=sys.stderr,
                creationflags=spawn_flags(),
                cwd=install,
                check=True,
            )


def installed(directory: Path, settings: dict, root: Path, target: str) -> None:
    """Install From Published Assets And Prove No Source Or Third-Party Fallback."""
    for item in definitions(directory, "release", target):
        if not item.get("installers"):
            continue
        version = settings["tag"].removeprefix("v")
        provider = settings["provider"]
        repository = settings.get("repository") if provider == "github" else settings["project_id"]
        for installer in item["installers"]:
            with temporary.directory() as install:
                if installer == "mise":
                    tool = f"{provider}:{repository}[asset_pattern={item['output']},bin={item['binary']}]@{version}"
                    environment = os.environ.copy()
                    environment.pop("MISE_ENABLE_TOOLS", None)
                    subprocess.run(
                        ["mise", "install-into", tool, str(install)],
                        stdin=sys.stdin,
                        stdout=sys.stdout,
                        stderr=sys.stderr,
                        creationflags=spawn_flags(),
                        cwd=install.parent,
                        env=environment,
                        check=True,
                    )
                    matches = list(install.rglob(item["binary"]))
                else:
                    command = [
                        "cargo",
                        "binstall",
                        item["package"],
                        "--version",
                        "=" + version,
                        "--target",
                        item["triple"],
                        "--root",
                        str(install),
                        "--no-confirm",
                        "--disable-strategies",
                        "quick-install,compile",
                        "--strategies",
                        "crate-meta-data",
                    ]
                    subprocess.run(
                        command,
                        stdin=sys.stdin,
                        stdout=sys.stdout,
                        stderr=sys.stderr,
                        creationflags=spawn_flags(),
                        cwd=install,
                        check=True,
                    )
                    matches = [install / "bin" / item["binary"]]
                if len(matches) != 1 or not matches[0].is_file():
                    raise policy.ToolError(f"{installer} did not install the declared executable")
                expected = hashlib.sha256(binary_bytes(root / item["output"], item["binary"])).digest()
                if hashlib.sha256(matches[0].read_bytes()).digest() != expected:
                    raise policy.ToolError(f"{installer} did not install the exact published asset")
                subprocess.run(
                    [str(matches[0]), *item["verify"]],
                    stdin=sys.stdin,
                    stdout=sys.stdout,
                    stderr=sys.stderr,
                    creationflags=spawn_flags(),
                    cwd=install,
                    check=True,
                )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=("dev", "release"), default="dev")
    parser.add_argument("--language", choices=("rust", "python"), required=True)
    parser.add_argument("--firmware-converter")
    options = parser.parse_args()
    product = paths.output(ROOT, "build", options.profile)
    destination = paths.output(ROOT, "package", options.profile) / options.language
    if options.language == "python":
        version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
        wheel(product / "python", paths.target(), version)
        paths.replace(ROOT, product / "python", destination)
        return

    executable = "helios-mesh.exe" if os.name == "nt" else "helios-mesh"
    with temporary.directory() as staging:
        shutil.copy2(product / "bin" / executable, staging / executable)
        if platform.system() == "Linux":
            if not options.firmware_converter:
                raise ValueError("Linux firmware packaging requires the pinned converter selection")
            environment = dict(os.environ)
            environment.pop("MISE_ENABLE_TOOLS", None)
            converter = subprocess.check_output(
                ["mise", "which", "elf2uf2-rs", "--tool", options.firmware_converter],
                env=environment,
                text=True,
                creationflags=spawn_flags(),
            ).strip()
            if not Path(converter).is_file():
                raise ValueError(f"the prepared firmware converter is missing: {options.firmware_converter}")
            firmware = staging / "firmware"
            firmware.mkdir()
            entries: dict[str, bytes] = {}
            entries["LICENSE"] = (ROOT / "LICENSE").read_bytes()
            for node in ("gateway", "rotator", "sidecar"):
                source = product / "firmware" / f"helios-mesh-node-{node}"
                image = firmware / f"helios-mesh-node-{node}.uf2"
                subprocess.run(
                    [converter, str(source), str(image)],
                    creationflags=spawn_flags(),
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
            packaged = ZipFile(firmware / "helios-mesh-firmware-feather-rp2040.zip", "w")
            try:
                for name, content in sorted(entries.items()):
                    info = ZipInfo(name)
                    info.create_system = 3
                    info.external_attr = 0o100644 << 16
                    info.compress_type = ZIP_DEFLATED
                    packaged.writestr(info, content)
            finally:
                packaged.close()
        paths.replace(ROOT, staging, destination, origin=staging)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
