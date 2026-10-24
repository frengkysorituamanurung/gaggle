# Building and Packaging Gaggle

This guide describes the packages produced by the GitHub Actions workflows and
how to reproduce them locally. Gaggle's UI is served by the Rust HTTP server;
there is no separate Electron, Tauri, Node.js, or native webview build.

## 1. Continuous integration

Workflow: `.github/workflows/ci.yml`

Triggers:

- Push to `development`
- Push to `main`
- Pull requests targeting `development` or `main`
- Merge queue checks for `development` or `main`
- Manual `workflow_dispatch`

The required job is `Format, Clippy & Test` on `ubuntu-latest` and
`macos-latest`. It runs:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
.github/scripts/check-ui-embedded.sh
bash -n .github/scripts/*.sh scripts/*.sh
```

The job names and matrix are intentionally unchanged because `main` branch
protection requires both status checks.

## 2. Pull request, development, and main package builds

Workflow: `.github/workflows/package.yml`

Every pull request targeting `development` or `main`, and every successful push
to either branch, builds native packages, runs a packaged server smoke test,
generates checksums, and uploads Actions artifacts. It does not create a GitHub
Release.

| Artifact | Runner | Target | Format |
| :--- | :--- | :--- | :--- |
| `Gaggle-macOS-arm64` | `macos-latest` | `aarch64-apple-darwin` | `.zip` |
| `Gaggle-macOS-x64` | `macos-15-intel` | `x86_64-apple-darwin` | `.zip` |
| `Gaggle-linux-x86_64` | `ubuntu-22.04` | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| `Gaggle-linux-aarch64` | `ubuntu-22.04-arm` | `aarch64-unknown-linux-gnu` | `.tar.gz` |

The Actions artifact wrapper is named with the full commit SHA, for example
`Gaggle-macOS-arm64-<sha>`. Inside each artifact you will find:

```text
Gaggle-macOS-arm64.zip
Gaggle-macOS-arm64.zip.sha256
Gaggle-macOS-arm64.json
```

and for Linux:

```text
Gaggle-linux-x86_64.tar.gz
Gaggle-linux-x86_64.tar.gz.sha256
Gaggle-linux-x86_64.json
```

A final workflow job downloads all four packages, requires the complete
four-package set, verifies every checksum, and uploads `build-manifest.json`
plus `SHA256SUMS` under `Gaggle-build-manifest-<sha>`.

Artifacts are pull request, development, and main builds, not stable releases.
They are retained for 14 days and are associated with the exact commit that
produced them. Pull request builds never receive signing or release secrets.

## 3. Reproduce package builds locally

Prerequisites:

- Rust stable with the 2024 edition
- `rustfmt` and Clippy for the full CI checks
- macOS packaging requires macOS
- Linux packaging requires a Linux host with `tar` and `sha256sum`

Build the release binaries:

```bash
cargo build --release --locked \
  -p gaggle-cli \
  --bin gaggle \
  --bin gaggle-desktop
```

For a target-specific build, add `--target <triple>` and use the corresponding
binary directory.

### macOS

The checked-in wrapper builds, signs, packages, checksums, and validates the
bundle in one step:

```bash
./scripts/package_macos_zip.sh
```

It uses the same `.github/scripts/package-macos.sh` and
`.github/scripts/validate-macos-bundle.sh` code as CI, defaults to ad-hoc
signing, and writes `dist/Gaggle-macOS-<arch>.zip` plus its `.sha256` file. To
use an existing release binary directory or a Developer ID identity:

```bash
./scripts/package_macos_zip.sh \
  --binary-dir "$PWD/target/aarch64-apple-darwin/release" \
  --sign-identity "Developer ID Application: Example (TEAMID)"
```

The longer manual equivalent uses the target-specific release directory because
`package-macos.sh` expects the exact target output:

```bash
MACOSX_DEPLOYMENT_TARGET=12.0 cargo build --release --locked \
  -p gaggle-cli \
  --bin gaggle \
  --bin gaggle-desktop \
  --target aarch64-apple-darwin

mkdir -p dist
.github/scripts/package-macos.sh \
  --target aarch64-apple-darwin \
  --version 0.1.0 \
  --binary-dir "$PWD/target/aarch64-apple-darwin/release" \
  --output-dir "$PWD/dist" \
  --commit "$(git rev-parse HEAD)"

.github/scripts/validate-macos-bundle.sh \
  --archive "$PWD/dist/Gaggle-macOS-arm64.zip" \
  --target aarch64-apple-darwin
```

### Linux

```bash
cargo build --release --locked \
  -p gaggle-cli \
  --bin gaggle \
  --bin gaggle-desktop \
  --target x86_64-unknown-linux-gnu

mkdir -p dist
.github/scripts/package-linux.sh \
  --target x86_64-unknown-linux-gnu \
  --version 0.1.0 \
  --binary-dir "$PWD/target/x86_64-unknown-linux-gnu/release" \
  --output-dir "$PWD/dist" \
  --commit "$(git rev-parse HEAD)" \
  --ref "$(git rev-parse HEAD)"

.github/scripts/validate-linux-package.sh \
  --archive "$PWD/dist/Gaggle-linux-x86_64.tar.gz" \
  --target x86_64-unknown-linux-gnu
```

## 4. Installing the macOS build

1. Download and extract `Gaggle-macOS-arm64.zip` or `Gaggle-macOS-x64.zip`.
2. Move `Gaggle.app` to `/Applications` or another writable application folder.
3. Open `Gaggle.app`.
4. The app starts a local server on `127.0.0.1:4321` and opens the default
   browser. If the browser does not open, visit
   <http://127.0.0.1:4321>.
5. The CLI is available inside the bundle at
   `Gaggle.app/Contents/Resources/bin/gaggle` if needed.

The committed icon at `assets/Gaggle.icns` is generated from
`docs/assets/gaggle-logo.svg` with `.github/scripts/generate-macos-icon.sh`;
packaging copies it into the bundle. The app does not store the database in the
bundle. The macOS database location is:

```text
~/Library/Application Support/gaggle/gaggle.db
```

Unsigned builds may show a Gatekeeper warning. Open the app with the standard
Finder confirmation flow. Do not disable Gatekeeper system-wide.

## 5. Installing the Linux build

```bash
tar -xzf Gaggle-linux-x86_64.tar.gz
cd Gaggle-linux-x86_64
./install.sh
```

`install.sh` installs into the current user's directories without `sudo`:

```text
~/.local/bin/gaggle
~/.local/bin/gaggle-desktop
~/.local/share/applications/gaggle.desktop
~/.local/share/icons/hicolor/scalable/apps/gaggle.svg
```

Make sure `~/.local/bin` is on `PATH`, then run:

```bash
gaggle-desktop
```

You can also run the package without installing it:

```bash
./bin/gaggle-desktop
```

The UI is opened in the default browser at <http://127.0.0.1:4321>. If
`xdg-open` is unavailable, open that URL manually.

The Linux database location is:

```text
${XDG_DATA_HOME:-$HOME/.local/share}/gaggle/gaggle.db
```

### Linux runtime requirements

- A glibc-based x86_64 or aarch64 Linux distribution. Packages are built and
  smoke-tested on Ubuntu 22.04, so the tested baseline is glibc 2.35 or newer.
- A desktop session with a browser for the desktop launcher.
- `xdg-open` for automatic browser startup.
- No Node.js, Electron, WebKit, or system webview runtime is required.

The portable archive is the only Linux format currently built and validated.
`.deb`, `.rpm`, and AppImage are not advertised until the project can build and
test them reliably.

## 6. Data, upgrades, and uninstall

The database is separate from the application package and is not removed when
the app or archive is replaced.

- macOS: `~/Library/Application Support/gaggle/gaggle.db`
- Linux: `${XDG_DATA_HOME:-$HOME/.local/share}/gaggle/gaggle.db`

Before upgrading, quit Gaggle and copy the database file while the process is
stopped. SQLite may also have `-wal` and `-shm` files; copy them with the
database if they exist. Pending database migrations run automatically on the
next startup. Do not assume that an older binary can open a database after a
newer version has migrated it; keep the backup until the upgrade is verified.

Uninstalling the application does not delete user data:

- macOS: quit Gaggle and remove `Gaggle.app`.
- Linux: remove `~/.local/bin/gaggle`, `~/.local/bin/gaggle-desktop`,
  `~/.local/share/applications/gaggle.desktop`, and
  `~/.local/share/icons/hicolor/scalable/apps/gaggle.svg`. Remove the database
  directory separately only when the data is no longer needed.

## 7. Stable releases

Workflow: `.github/workflows/release.yml`

A stable release is published only by pushing a tag matching the workspace
version. Pull requests and ordinary branch pushes never publish a release;
they only upload workflow artifacts. The tagged commit must already be on
`main`:

```bash
git checkout main
git pull --ff-only
git tag -a v0.1.0 -m "Gaggle 0.1.0"
git push origin v0.1.0
```

Before building, the workflow verifies:

- the tag is exactly `v<workspace version>`;
- the tag points at the checked-out commit;
- the commit is an ancestor of `origin/main`;
- the working tree is clean.

The tagged commit runs the same formatting, lint, compilation, test,
embedded-UI, and shell-syntax checks as CI. All native packages must build and
validate before the publish job runs. The publish job:

- downloads the packages from the same workflow run;
- verifies every SHA-256 file;
- creates `build-manifest.json` and `SHA256SUMS`;
- generates release notes from Git history;
- creates or updates the GitHub Release for the tag.

The publish job uses the repository `GITHUB_TOKEN` with `contents: write`. The
build jobs keep `contents: read`.

## 8. macOS signing and notarization

Signing is optional. It is enabled only when all of these repository secrets
are configured:

- `APPLE_CERTIFICATE_BASE64`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_ID`
- `APPLE_ID_PASSWORD`
- `APPLE_TEAM_ID`

The release workflow checks for all five before enabling signing. If they are
missing, the release continues with unsigned bundles and the metadata records
`signed: false` and `notarized: false`.

When enabled, the workflow imports the certificate into a temporary keychain,
signs the nested executables and app with the hardened runtime, submits the ZIP
to `notarytool`, staples the app, and recreates the ZIP with `ditto
--sequesterRsrc` so notarization metadata survives distribution. It never
disables Gatekeeper and does not claim an unsigned build is notarized.

## 9. Workflow and release limitations

- CI validates the macOS bundle by extracting the ZIP and starting
  `Contents/MacOS/Gaggle --no-browser`; this is not automated Finder GUI
  interaction. The local audit also launched the bundle through LaunchServices
  with `--args --no-browser`.
- Linux packages are built and smoke-tested on native Ubuntu 22.04 GitHub
  runners. Local validation requires a Linux host or container runtime.
- The `ubuntu-22.04-arm` and `macos-15-intel` runner labels must be confirmed
  by the first GitHub Actions run. Do not silently substitute unvalidated
  cross-compiled artifacts if a label is unavailable.
- Stable releases are not created from ordinary branch pushes or pull requests.
- There is no Windows package in this pipeline.
- The application is a local browser UI, not a native webview. Users need a
  browser and a loopback-capable environment.
- Release provenance attestations are not enabled yet. CodeQL, Dependabot, and
  SHA-256 checksums are the current supply-chain controls; `cargo audit` still
  reports the findings documented in the CI/CD audit.

## 10. Related documentation

- [CI/CD audit and comparison](./ci-cd-audit.md)
- [Repository README](../../README.md)
- [Contributing guide](../../CONTRIBUTING.md)
- [Database ERD](../ERD.md)
