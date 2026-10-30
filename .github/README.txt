# Release procedure

The `Release installers` workflow in `workflows/installers.yml` runs whenever
a tag is pushed. Branch pushes alone do not create releases.

## Prerequisites

- GitHub Actions must be enabled on the repository, with permission for the
  publishing job's `GITHUB_TOKEN` to write repository contents/releases.
- The `origin` remote must point to the GitHub repository you want to release.
- The tagged commit must contain the installer workflow, the `packaging/`
  files, and all intended application/documentation changes.
- Use Rust 1.88 or newer and Python 3.11 or newer for local preparation.
  Desktop builds also need the platform dependencies listed in ../README.md.

No signing credentials are required. macOS applications are ad-hoc signed,
not Developer ID signed or notarized; Windows installers are unsigned.
Gatekeeper/SmartScreen may warn users. FFmpeg is not bundled.

## 1. Choose a version

Use SemVer and increase the version for each new release:

- Stable release: Cargo version `0.2.0`, tag `v0.2.0`.
- Prerelease: Cargo version `0.2.0-rc.1`, tag `v0.2.0-rc.1`.

The `v` prefix is optional, but the rest of the tag must exactly match
`[package].version` in `Cargo.toml`. Do not include `v` in the Cargo version.
Prereleases are automatically marked as prereleases on GitHub.

Installer versions are limited to major/minor values of 0-255 and patch
values of 0-65535. Numeric prerelease identifiers cannot have leading zeros.
Windows MSI compares only the numeric version core: a prerelease and final
release with the same core can replace one another. Build metadata does not
provide a separate RPM/MSI upgrade version. Prefer increasing the numeric
core rather than relying on metadata to distinguish upgrades.

## 2. Update Cargo files and check the release

For example, change `[package].version` in `Cargo.toml` to `0.2.0`.
Run Cargo to refresh the root package version in `Cargo.lock` without
deliberately upgrading dependencies:

```sh
cargo check
git diff -- Cargo.toml Cargo.lock
```

Commit both files. Do not edit only `Cargo.toml`: the workflow uses
`cargo --locked`, so an outdated lockfile will fail the build.

Before committing, run the local checks with the required desktop dependencies
installed:

```sh
cargo fmt --all -- --check
cargo test --locked
python3 -m unittest discover -s packaging -p 'test_*.py'
python3 packaging/package.py metadata --tag v0.2.0
```

Update relevant documentation and ensure all intended release changes are
committed. For the version bump itself:

```sh
git add Cargo.toml Cargo.lock
git commit -m "Release 0.2.0"
git status --short
```

Resolve any remaining uncommitted release changes before tagging: only the
tagged commit is built, not local working-tree files.

## 3. Push the commit and matching tag

Replace `0.2.0` in all commands with the chosen version:

```sh
git push origin HEAD
git tag -a v0.2.0 -m "IBNIZ Rust 0.2.0"
git push origin v0.2.0
```

Push the tag explicitly; a normal branch push does not push tags. Do not
reuse or move an existing published release tag.

## 4. Follow publication

Open the repository's Actions page and select the `Release installers` run
for the tag. The workflow validates the version, builds the desktop binaries,
and packages/checks all five installers:

| Platform | Assets |
|----------|--------|
| Debian / Ubuntu x86-64 | One `.deb` |
| Fedora x86-64 | One `.rpm` |
| macOS Intel and Apple Silicon | Two `.dmg` files |
| Windows x86-64 | One `.msi` |

All build jobs must succeed before publication. Installers are also retained
as workflow artifacts. The publishing job generates `SHA256SUMS`, creates a
draft release if needed, uploads all assets, and publishes the release.
Release notes are generated automatically; review/edit them on GitHub.

Confirm the Releases page contains the five installers and `SHA256SUMS`,
with the correct version and prerelease status. Download the assets and check
their checksums before distributing them.

## Failed releases

Inspect the failed job's logs on the Actions page. No new release is published
until all builds succeed; an upload failure may leave a draft release.

For a transient runner/network failure, rerun all jobs for the same workflow
run. Reruns replace same-named release assets, including on an already
published release, so do not use reruns to distribute different source code.

If source, packaging, or version files need fixing, commit the fix and choose
a new matching version/tag. Rerunning an old run still builds its original
tagged commit.
