# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Prove Firmware Packaging Uses The Prepared Converter And Existing Build Bytes."""

# === Imports === #

# Standard Library Imports
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from zipfile import ZipFile

# Third Party Imports

# Local Imports
from helios_mesh_tools.package import product

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #


class PackageTests(unittest.TestCase):
    def test_python_package_uses_one_matching_built_wheel(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        (root / "Cargo.toml").write_text('[workspace.package]\nversion = "0.1.5"\n')
        for selected, tag in (("linux-x86_64", "manylinux_2_39_x86_64"), ("windows-x86_64", "win_amd64")):
            built = root / "target/build/dev" / selected / "python"
            built.mkdir(parents=True)
            name = f"helios_mesh-0.1.5-cp310-abi3-{tag}.whl"
            (built / name).write_bytes(b"built wheel")
            with (
                patch.object(product, "ROOT", root),
                patch.object(product.paths, "target", return_value=selected),
                patch("sys.argv", ["package", "--language", "python"]),
                patch.object(product.subprocess, "run") as compile_command,
            ):
                product.main()
            self.assertEqual((root / "target/package/dev" / selected / "python" / name).read_bytes(), b"built wheel")
            compile_command.assert_not_called()

    def test_rejects_incomplete_or_incompatible_wheel_sets(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        directory = Path(temporary.name)
        cases = (
            (),
            ("helios_mesh-0.1.5-cp315-cp315-win_amd64.whl",),
            ("helios_mesh-0.1.5-cp310-abi3-linux_x86_64.whl",),
            ("helios_mesh-0.1.4-cp310-abi3-win_amd64.whl",),
            ("helios_mesh-0.1.5-cp310-abi3-win_amd64.whl", "helios_mesh-0.1.5-cp315-cp315-win_amd64.whl"),
        )
        for index, names in enumerate(cases):
            built = directory / str(index)
            built.mkdir()
            for name in names:
                (built / name).write_bytes(b"wheel")
            with self.subTest(names=names), self.assertRaises(ValueError):
                product.wheel(built, "windows-x86_64", "0.1.5")

    def test_pinned_converter(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        (root / "LICENSE").write_bytes(b"license fixture")
        (root / "Cargo.toml").write_text('[workspace.package]\nversion = "0.1.5"\n')
        converter = root / "converter"
        converter.write_bytes(b"prepared converter")
        built = root / "target/build/dev/fixture"
        (built / "bin").mkdir(parents=True)
        for name in ("helios-mesh", "helios-mesh.exe"):
            (built / "bin" / name).write_bytes(b"host fixture")
        (built / "firmware").mkdir()
        roles = ("gateway", "rotator", "sidecar")
        for index, role in enumerate(roles):
            (built / "firmware" / f"helios-mesh-node-{role}").write_bytes(bytes([index]))

        def output(directory: Path, stage: str, profile: str) -> Path:
            return root / "target" / stage / profile / "fixture"

        def convert(command: list[str], **kwargs: object) -> None:
            self.assertEqual(command[0], str(converter))
            source, destination = map(Path, command[1:])
            destination.write_bytes(b"UF2" + source.read_bytes())

        self.enterContext(patch.object(product, "spawn_flags", return_value=12345))
        with (
            patch.object(product, "ROOT", root),
            patch.object(product.paths, "output", side_effect=output),
            patch.object(product.platform, "system", return_value="Linux"),
            patch.object(product.subprocess, "check_output", return_value=str(converter)) as lookup,
            patch.object(product.subprocess, "run", side_effect=convert) as conversion,
            patch.dict(os.environ, {"MISE_ENABLE_TOOLS": "python,uv"}),
            patch("sys.argv", ["package", "--language", "rust", "--firmware-converter", "cargo:converter@1.2.3"]),
        ):
            product.main()
        self.assertNotIn("MISE_ENABLE_TOOLS", lookup.call_args.kwargs["env"])
        self.assertEqual(lookup.call_args.args[0][-2:], ["--tool", "cargo:converter@1.2.3"])
        self.assertEqual(conversion.call_count, len(roles))
        self.assertEqual(lookup.call_args.kwargs["creationflags"], 12345)
        for call in conversion.call_args_list:
            self.assertEqual(call.kwargs["creationflags"], 12345)
        with ZipFile(root / "target/package/dev/fixture/rust/firmware/helios-mesh-firmware-feather-rp2040.zip") as archive:
            for index, role in enumerate(roles):
                self.assertEqual(archive.read(f"helios-mesh-node-{role}.uf2"), b"UF2" + bytes([index]))
                self.assertEqual(archive.read(f"debug/helios-mesh-node-{role}.elf"), bytes([index]))
            self.assertEqual(archive.read("LICENSE"), b"license fixture")


# === Public Functions === #

# === Private Functions === #

# === Entry Point === #
