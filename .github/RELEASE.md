# Release procedure

Pushing a tag starts the `Release installers` workflow in
`workflows/installers.yml`. A branch push alone does not create a release.

## Prerequisites

- Enable GitHub Actions. Allow the publishing job's `GITHUB_TOKEN` to write
  repository contents and releases.
- Point `origin` to the repository you want to release.
- Include the installer workflow, `packaging/` files, and all intended code
  and documentation changes in the tagged commit.
- Use Rust 1.88 or newer and Python 3.11 or newer for local preparation.
  Desktop builds also need the [platform dependencies](../DOCUMENTATION.md#run).

No signing credentials are required. macOS apps are ad-hoc signed, not
Developer ID signed or notarized. Windows installers are unsigned.
Gatekeeper/SmartScreen may warn users.

FFmpeg is not bundled.

## 1. Choose a version

Use SemVer. Increase the version for each release:

- Stable release: Cargo version `0.2.0`, tag `v0.2.0`.
- Prerelease: Cargo version `0.2.0-rc.1`, tag `v0.2.0-rc.1`.

The `v` prefix is optional, but the rest of the tag must exactly match
`[package].version` in `Cargo.toml`. Do not include `v` in the Cargo version.
Prereleases are automatically marked as prereleases on GitHub.

**Installer version limits:** major and minor must be 0-255; patch must be
0-65535. Numeric prerelease identifiers cannot have leading zeros.

Windows MSI compares only the numeric version core. A prerelease and final
release with the same core can replace one another. Build metadata does not
create a separate RPM/MSI upgrade version. Increase the numeric core to
distinguish upgrades.

## 2. Update Cargo files and check the release

For example, change `[package].version` in `Cargo.toml` to `0.2.0`.
Run Cargo to refresh the root package version in `Cargo.lock` without
deliberately upgrading dependencies:

```sh
cargo check
git diff -- Cargo.toml Cargo.lock
```

Commit both files. The workflow uses `cargo --locked`; an outdated lockfile
fails the build. Do not update only `Cargo.toml`.

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

Commit all release changes before tagging. The build uses the tagged commit,
not local working-tree files.

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

Open **Actions** and select the tag's `Release installers` run. The workflow
validates the version, builds the desktop binaries, and packages and checks
all five installers:

| Platform | Assets |
|----------|--------|
| Debian / Ubuntu x86-64 | One `.deb` |
| Fedora x86-64 | One `.rpm` |
| macOS Intel and Apple Silicon | Two `.dmg` files |
| Windows x86-64 | One `.msi` |

All build jobs must succeed before publication. Installers are also kept as
workflow artifacts.

The publishing job generates `SHA256SUMS`, creates a draft if needed, uploads
all assets, and publishes the release. Review the generated release notes
on GitHub.

Confirm the Releases page contains the five installers and `SHA256SUMS`,
with the correct version and prerelease status. Download the assets and check
their checksums before distributing them.

## Failed releases

Inspect the failed job's logs on the Actions page. No new release is published
until all builds succeed; an upload failure may leave a draft release.

For a temporary runner or network failure, rerun all jobs in the same workflow
run. **Reruns replace same-named assets**, even on published releases.
Do not use a rerun to distribute different source code.

If source, packaging, or version files need fixing, commit the fix and choose
a new matching version/tag. Rerunning an old run still builds its original
tagged commit.
