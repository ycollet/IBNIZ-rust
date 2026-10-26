import hashlib
from pathlib import Path
import plistlib
import struct
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

import package


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="ibniz-packaging-test-")
        self.addCleanup(self.temporary.cleanup)
        self.work = Path(self.temporary.name)
        self.manifest = self.work / "Cargo.toml"
        self.manifest.write_text('[package]\nversion = "0.1.0"\n', encoding="utf-8")
        self.binary = self.work / "ibniz"
        self.binary.write_bytes(b"test executable")
        self.version = package.release_version("v0.1.0", self.manifest)

    def test_versions_match_cargo_and_translate_prereleases(self):
        self.assertEqual(package.release_version("0.1.0", self.manifest), self.version)
        self.assertFalse(self.version["prerelease"])
        self.manifest.write_text('[package]\nversion = "1.2.3-rc.1+build.4"\n', encoding="utf-8")
        version = package.release_version("v1.2.3-rc.1+build.4", self.manifest)
        self.assertEqual(version["core"], "1.2.3")
        self.assertEqual(version["deb_version"], "1.2.3~rc.1+build.4")
        self.assertEqual(version["rpm_release"], "0.1.rc.1")
        self.assertTrue(version["prerelease"])

    def test_invalid_tags_and_mismatched_versions_fail(self):
        for tag in ["latest", "v1", "v1.2.3\n", "v01.2.3", "v1.2.3-01",
                    "v1.2.3-rc..1", "v1.2.3+bad..build", "v256.0.0",
                    "v1.256.0", "v1.0.65536", "v0.2.0", 'v0.1.0"; echo bad']:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                package.release_version(tag, self.manifest)

    def test_linux_payload_installs_binary_icons_license_and_launcher(self):
        stage = self.work / "root"
        package.linux_files(self.binary, stage)
        self.assertEqual((stage / "usr/bin/ibniz").read_bytes(), self.binary.read_bytes())
        self.assertTrue((stage / "usr/bin/ibniz").stat().st_mode & 0o111)
        self.assertEqual((stage / "usr/share/doc/ibniz/LICENSE").read_bytes(),
                         (package.ROOT / "LICENSE").read_bytes())
        self.assertIn("Exec=ibniz %f", (stage / "usr/share/applications/ibniz.desktop").read_text())
        self.assertTrue((stage / "usr/share/icons/hicolor/scalable/apps/ibniz.svg").is_file())

    def test_debian_dependencies_and_control_are_generated_from_the_binary(self):
        output = self.work / "package.deb"
        with patch.object(package.subprocess, "check_output",
                          return_value="shlibs:Depends=libc6 (>= 2.36), libasound2 (>= 1.0)\n"), \
                patch.object(package, "run") as command:
            package.deb(self.binary, self.work, output, self.version)
        control = (self.work / "root/DEBIAN/control").read_text()
        self.assertIn("Version: 0.1.0\n", control)
        self.assertIn("Architecture: amd64\n", control)
        self.assertIn("libc6 (>= 2.36), libasound2 (>= 1.0)", control)
        self.assertIn("libwayland-client0", control)
        self.assertTrue((self.work / "root/usr/share/doc/ibniz/copyright").is_file())
        self.assertIn("--root-owner-group", command.call_args_list[0].args)

    def test_rpm_selects_only_the_generated_binary_package(self):
        output = self.work / "package.rpm"

        def command(*args, **kwargs):
            if args[0] == "rpmbuild":
                generated = self.work / "rpm/RPMS/x86_64/ibniz-0.1.0-1.x86_64.rpm"
                generated.parent.mkdir()
                generated.write_bytes(b"test rpm")

        with patch.object(package, "run", side_effect=command):
            package.rpm(self.binary, self.work, output, self.version)
        self.assertEqual(output.read_bytes(), b"test rpm")

    def test_mac_bundle_metadata_executable_and_applications_link(self):
        with patch.object(package, "run"):
            package.dmg(self.binary, self.work, self.work / "package.dmg", self.version)
        contents = self.work / "image/IBNIZ Rust.app/Contents"
        with (contents / "Info.plist").open("rb") as file:
            metadata = plistlib.load(file)
        self.assertEqual(metadata["CFBundleExecutable"], "ibniz")
        self.assertEqual(metadata["CFBundleShortVersionString"], "0.1.0")
        self.assertTrue((contents / "MacOS/ibniz").stat().st_mode & 0o111)
        self.assertEqual((self.work / "image/Applications").readlink(), Path("/Applications"))

    def test_windows_icon_rtf_and_wix_payload_are_well_formed(self):
        with patch.object(package, "run") as command:
            package.msi(self.binary, self.work, self.work / "package.msi", self.version)
        icon = (self.work / "ibniz.ico").read_bytes()
        self.assertEqual(struct.unpack("<HHH", icon[:6]), (0, 1, 1))
        self.assertEqual(icon[22:], (package.ROOT / "assets/ibniz.png").read_bytes())
        self.assertTrue((self.work / "license.rtf").read_text().startswith(r"{\rtf1"))
        self.assertIn("WixToolset.UI.wixext/5.0.2", command.call_args.args)
        namespace = {"w": "http://wixtoolset.org/schemas/v4/wxs"}
        tree = ET.parse(package.ROOT / "packaging/ibniz.wxs")
        product = tree.find("w:Package", namespace)
        self.assertEqual(product.attrib["Scope"], "perMachine")
        self.assertIsNotNone(product.find(".//w:Environment[@Name='PATH']", namespace))
        shortcut = product.find(".//w:Shortcut", namespace)
        self.assertEqual(Path(shortcut.attrib["Icon"]).suffix, ".exe")

    def test_checksums_require_all_five_installers_and_hash_their_bytes(self):
        with self.assertRaises(ValueError):
            package.checksums(self.work)
        names = ["a.deb", "b.rpm", "c-x86_64.dmg", "c-arm64.dmg", "d.msi"]
        for name in names:
            (self.work / name).write_bytes(name.encode("ascii"))
        package.checksums(self.work)
        expected = "".join(f"{hashlib.sha256(name.encode('ascii')).hexdigest()}  {name}\n"
                           for name in sorted(names))
        self.assertEqual((self.work / "SHA256SUMS").read_text(), expected)


if __name__ == "__main__":
    unittest.main()
