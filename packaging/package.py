"""Build native installers using only Python's standard library and platform tools."""

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import struct
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
FORMATS = {"deb": "linux-x86_64.deb", "rpm": "linux-x86_64.rpm",
           "dmg": "macos-{arch}.dmg", "msi": "windows-x86_64.msi"}
MAINTAINER = "IBNIZ Rust contributors <noreply@users.noreply.github.com>"


def release_version(tag, manifest=ROOT / "Cargo.toml"):
    version = tag.removeprefix("v")
    match = re.fullmatch(
        r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
        r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
        r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?", version,
    )
    if not match:
        raise ValueError("Tags must be SemVer versions, for example v0.2.0 or v0.2.0-rc.1")
    major, minor, patch = map(int, match.group(1, 2, 3))
    prerelease, metadata = match.group(4, 5)
    if prerelease and any(part.isdigit() and len(part) > 1 and part[0] == "0"
                          for part in prerelease.split(".")):
        raise ValueError("Numeric prerelease identifiers must not have leading zeroes")
    if major > 255 or minor > 255 or patch > 65535:
        raise ValueError("Installer versions must fit Windows MSI's 255.255.65535 limit")
    with manifest.open("rb") as file:
        cargo_version = tomllib.load(file)["package"]["version"]
    if cargo_version != version:
        raise ValueError(f"Tag version {version} does not match Cargo.toml version {cargo_version}")
    core = f"{major}.{minor}.{patch}"
    return {
        "version": version,
        "core": core,
        "prerelease": bool(prerelease),
        "deb_version": core + (f"~{prerelease}" if prerelease else "")
        + (f"+{metadata}" if metadata else ""),
        "rpm_release": f"0.1.{prerelease.replace('-', '.')}" if prerelease else "1",
    }


def run(*args, **kwargs):
    subprocess.run([str(arg) for arg in args], check=True, **kwargs)


def copy(source, destination, executable=False):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)
    destination.chmod(0o755 if executable else 0o644)


def linux_files(binary, stage):
    copy(binary, stage / "usr/bin/ibniz", executable=True)
    copy(ROOT / "assets/ibniz.desktop", stage / "usr/share/applications/ibniz.desktop")
    copy(ROOT / "assets/ibniz.svg", stage / "usr/share/icons/hicolor/scalable/apps/ibniz.svg")
    copy(ROOT / "assets/ibniz.png", stage / "usr/share/icons/hicolor/256x256/apps/ibniz.png")
    for name in ["LICENSE", "README.md"]:
        copy(ROOT / name, stage / "usr/share/doc/ibniz" / name)


def deb(binary, work, output, version):
    stage = work / "root"
    linux_files(binary, stage)
    (stage / "usr/share/doc/ibniz/LICENSE").rename(stage / "usr/share/doc/ibniz/copyright")
    (work / "debian").mkdir()
    (work / "debian/control").write_text(
        f"Source: ibniz\nSection: sound\nPriority: optional\nMaintainer: {MAINTAINER}\n\n"
        "Package: ibniz\nArchitecture: any\nDescription: IBNIZ audiovisual virtual machine\n",
        encoding="utf-8",
    )
    dependencies = subprocess.check_output(
        ["dpkg-shlibdeps", "-O", "-e" + str(stage / "usr/bin/ibniz")],
        cwd=work, text=True,
    ).strip()
    if not dependencies.startswith("shlibs:Depends="):
        raise ValueError(f"Unexpected dpkg-shlibdeps output: {dependencies}")
    runtime = (
        "libx11-6, libxcursor1, libxi6, libxrandr2, libxinerama1, libgl1, libegl1, "
        "libwayland-client0, libwayland-cursor0, libwayland-egl1, "
        "libxkbcommon0, libxkbcommon-x11-0"
    )
    size = sum(file.stat().st_size for file in stage.rglob("*") if file.is_file())
    (stage / "DEBIAN").mkdir()
    (stage / "DEBIAN/control").write_text(
        f"Package: ibniz\nVersion: {version['deb_version']}\nArchitecture: amd64\n"
        f"Maintainer: {MAINTAINER}\nSection: sound\nPriority: optional\n"
        f"Installed-Size: {(size + 1023) // 1024}\n"
        f"Depends: {dependencies.removeprefix('shlibs:Depends=')}, {runtime}\n"
        "Description: Live audiovisual programming with the IBNIZ virtual machine\n"
        " An altered Rust implementation with a live editor, audio/video playback,\n"
        " recording, bundled examples, a VM debugger and NSM session support.\n",
        encoding="utf-8",
    )
    run("dpkg-deb", "--root-owner-group", "--build", stage, output)
    run("dpkg-deb", "--info", output)


def rpm(binary, work, output, version):
    stage = work / "root"
    linux_files(binary, stage)
    top = work / "rpm"
    for directory in ["BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"]:
        (top / directory).mkdir(parents=True, exist_ok=True)
    run("rpmbuild", "-bb", ROOT / "packaging/ibniz.spec", "--define", f"_topdir {top}",
        "--define", f"stagedir {stage}", "--define", f"package_version {version['core']}",
        "--define", f"package_release {version['rpm_release']}")
    packages = list((top / "RPMS").rglob("*.rpm"))
    if len(packages) != 1:
        raise ValueError(f"Expected one RPM, got {packages}")
    shutil.copyfile(packages[0], output)
    run("rpm", "-qpi", output)


def dmg(binary, work, output, version):
    image = work / "image"
    app = image / "IBNIZ Rust.app"
    contents = app / "Contents"
    copy(binary, contents / "MacOS/ibniz", executable=True)
    resources = contents / "Resources"
    for name in ["LICENSE", "README.md"]:
        copy(ROOT / name, resources / name)
    iconset = work / "ibniz.iconset"
    iconset.mkdir()
    for size in [16, 32, 128, 256, 512]:
        for scale in [1, 2]:
            suffix = "@2x" if scale == 2 else ""
            run("sips", "-z", size * scale, size * scale, ROOT / "assets/ibniz.png",
                "--out", iconset / f"icon_{size}x{size}{suffix}.png",
                stdout=subprocess.DEVNULL)
    run("iconutil", "-c", "icns", iconset, "-o", resources / "ibniz.icns")
    with (contents / "Info.plist").open("wb") as file:
        plistlib.dump({
            "CFBundleIdentifier": "org.ibniz-rust.ibniz",
            "CFBundleName": "IBNIZ Rust",
            "CFBundleDisplayName": "IBNIZ Rust",
            "CFBundleExecutable": "ibniz",
            "CFBundleIconFile": "ibniz.icns",
            "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": version["core"],
            "CFBundleVersion": version["core"],
            "LSMinimumSystemVersion": "11.0",
            "LSApplicationCategoryType": "public.app-category.music",
            "NSHighResolutionCapable": True,
            "NSSupportsAutomaticGraphicsSwitching": True,
        }, file)
    # Ad-hoc signing seals the bundle but does not confer Developer ID trust.
    run("codesign", "--force", "--sign", "-", app)
    run("codesign", "--verify", "--strict", app)
    (image / "Applications").symlink_to("/Applications", target_is_directory=True)
    run("hdiutil", "create", "-volname", "IBNIZ Rust", "-srcfolder", image,
        "-format", "UDZO", output)
    run("hdiutil", "verify", output)


def msi(binary, work, output, version):
    png = (ROOT / "assets/ibniz.png").read_bytes()
    icon = work / "ibniz.ico"
    icon.write_bytes(struct.pack("<HHHBBBBHHII", 0, 1, 1, 0, 0, 0, 0, 1, 32, len(png), 22) + png)
    license_text = (ROOT / "LICENSE").read_text(encoding="ascii")
    escaped = license_text.replace("\\", "\\\\").replace("{", "\\{").replace("}", "\\}")
    license_rtf = work / "license.rtf"
    license_rtf.write_text(r"{\rtf1\ansi{\fonttbl{\f0 Arial;}}\f0\fs20 "
                           + escaped.replace("\n", "\\par\n") + "}", encoding="ascii")
    run("wix", "build", ROOT / "packaging/ibniz.wxs", "-arch", "x64",
        "-ext", "WixToolset.UI.wixext/5.0.2", "-d", f"BinaryPath={binary}",
        "-d", f"RepoRoot={ROOT}", "-d", f"IconPath={icon}",
        "-d", f"LicensePath={license_rtf}", "-d", f"Version={version['core']}",
        "-intermediateFolder", work / "wix", "-o", output)


def checksums(directory):
    files = sorted(file for file in directory.iterdir() if file.suffix in {".deb", ".rpm", ".dmg", ".msi"})
    if Counter(file.suffix for file in files) != {".deb": 1, ".rpm": 1, ".dmg": 2, ".msi": 1}:
        raise ValueError("Expected five installers: DEB, RPM, two DMGs and MSI")
    lines = []
    for file in files:
        with file.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        lines.append(f"{digest}  {file.name}\n")
    text = "".join(lines)
    (directory / "SHA256SUMS").write_text(text, encoding="ascii")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    metadata = sub.add_parser("metadata")
    metadata.add_argument("--tag", required=True)
    build = sub.add_parser("build")
    build.add_argument("--tag", required=True)
    build.add_argument("--format", choices=FORMATS, required=True)
    build.add_argument("--arch", choices=["x86_64", "arm64"], default="x86_64")
    build.add_argument("--binary", type=Path, required=True)
    build.add_argument("--output", type=Path, default=ROOT / "target/installers")
    checksum = sub.add_parser("checksums")
    checksum.add_argument("directory", type=Path)
    args = parser.parse_args()
    if args.command == "checksums":
        checksums(args.directory)
        return
    version = release_version(args.tag)
    if args.command == "metadata":
        print(json.dumps(version))
        if "GITHUB_OUTPUT" in os.environ:
            with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as file:
                file.write(f"version={version['version']}\nprerelease={str(version['prerelease']).lower()}\n")
        return
    if args.format != "dmg" and args.arch != "x86_64":
        parser.error("Only macOS installers currently support arm64")
    binary = args.binary.resolve(strict=True)
    args.output.mkdir(parents=True, exist_ok=True)
    output = args.output.resolve() / f"ibniz-{version['version']}-{FORMATS[args.format].format(arch=args.arch)}"
    if output.exists():
        raise FileExistsError(f"Refusing to overwrite {output}")
    with tempfile.TemporaryDirectory(prefix="ibniz-package-") as temporary:
        {"deb": deb, "rpm": rpm, "dmg": dmg, "msi": msi}[args.format](
            binary, Path(temporary), output, version)
    if not output.is_file() or output.stat().st_size == 0:
        raise ValueError(f"Packaging did not create a nonempty installer: {output}")
    print(f"Created {output}")


if __name__ == "__main__":
    main()
