# CI/CD, Cross-Platform Packaging, and Release Audit

Status: implementation audit for the CI/CD work on `development`.
Audit date: 2026-10-24.

This document records the evidence used to design Gaggle's CI/CD pipeline. It
compares the repository with the relevant Goose workflows, lists the gaps that
were found, and explains the implementation choices. It intentionally does not
claim that a workflow has run on GitHub until code is pushed and a workflow run
can be inspected.

## 1. Repository facts verified during the audit

| Fact | Evidence |
| :--- | :--- |
| Git repository | `frengkysorituamanurung/gaggle` |
| Integration branch | `development` exists and is the merge target for feature PRs |
| Stable branch | `main` exists and is the default branch |
| Workspace | Cargo workspace with eight crates: `gaggle-core`, `gaggle-db`, `gaggle-tools`, `gaggle-mcp`, `gaggle-recipes`, `gaggle-server`, `gaggle-cli`, `gaggle-ui` |
| Workspace version | `0.1.0` in `Cargo.toml` |
| Rust edition | 2024 (`edition = "2024"`) |
| Binary | `gaggle` from `crates/gaggle-cli` |
| Desktop UI | Embedded HTML/CSS/JavaScript served by `gaggle-server`; no Tauri, Electron, GTK, WebKit, egui, or other native desktop framework dependency was found |
| Frontend build | None; `gaggle-ui::render_index_html()` returns the embedded UI |
| Database | Global SQLite database, created outside the application bundle |
| Existing icon asset | `docs/assets/gaggle-logo.svg` |
| Existing release assets | None; `dist/` was empty and untracked |
| Existing packaging scripts | None |
| Existing toolchain pin | None before this work |
| Existing Cargo config | No `.cargo/config.toml` |

The absence of a desktop webview is a deliberate architectural constraint, not
an omission. The packaged desktop experience therefore wraps the local HTTP
server with a native launcher and opens the user's browser. The UI itself is
already shipped inside the Rust binary.

## 2. Branch protection evidence

The GitHub API reports that `main` requires these status checks:

```text
Format, Clippy & Test (ubuntu-latest)
Format, Clippy & Test (macos-latest)
```

The required-check names are produced by the existing matrix job name in
`.github/workflows/ci.yml`. The implementation preserves that job name and
matrix so branch protection does not break.

`development` also exists as the integration branch. CI and the package matrix
trigger on pushes and pull requests for both `development` and `main`. Pull
request package builds are artifact-only and never publish a release.

## 3. Current Gaggle workflow inventory (before this change)

| File | Triggers | Jobs and purpose | Permissions | Gaps |
| :--- | :--- | :--- | :--- | :--- |
| `.github/workflows/ci.yml` | Push and PR to `main`, `development` | `Format, Clippy & Test` on Ubuntu and macOS: `cargo fmt`, `cargo clippy`, `cargo test` | `contents: read` | No manual trigger, no merge queue, no concurrency cancellation, no explicit `cargo check`, no `--locked`, no UI syntax check, actions not pinned to full SHAs |
| `.github/workflows/codeql.yml` | Push and PR to `main`, `development`; weekly schedule | CodeQL Rust analysis | `actions: read`, `contents: read`, `security-events: write` | Actions use major tags rather than full SHAs; no concurrency group |
| `.github/workflows/pr-summary.yml` | PR opened/synchronized/reopened targeting `main` | Rewrites promotion PR body when head is `development` | `contents: read`, `pull-requests: write` | Limited to promotion PRs; not a release pipeline |
| `.github/workflows/branch-cleanup.yml` | PR closed against `main`, `development` | Deletes ephemeral head branches | `contents: write` | Write permission is scoped to cleanup; guarded by same-repository and non-protected-branch checks |
| `.github/dependabot.yml` | Weekly schedule | Cargo and GitHub Actions dependency updates | N/A | No `cargo audit`/`cargo deny` policy; update groups target `development` |

No workflow built or uploaded application packages, generated checksums,
published stable releases, or ran a packaged application. There was no
Windows workflow and no macOS `.dmg` workflow.

## 4. Reference material inspected from Goose

The audit used the following files from `aaif-goose/goose` at the repository
head `c3bfc5f7b0e230180ba3164ec1ee84fd951eaf0d` (2026-10-23). The individual
file commit SHAs are recorded because the workflows are not frozen.

| Reference | URL | Inspected commit |
| :--- | :--- | :--- |
| CI workflow | <https://github.com/aaif-goose/goose/blob/main/.github/workflows/ci.yml> | `1de2f75742457e3cf6350426e20ab8d3a579d4a8` |
| Release workflow | <https://github.com/aaif-goose/goose/blob/main/.github/workflows/release.yml> | `da6367f604aa255da7baa64ed57963bbee273455` |
| Canary workflow | <https://github.com/aaif-goose/goose/blob/main/.github/workflows/canary.yml> | `da6367f604aa255da7baa64ed57963bbee273455` |
| Linux build notes | <https://github.com/aaif-goose/goose/blob/main/BUILDING_LINUX.md> | `a0fa41dd7642d1761d3fd23cb126a610fc434aa9` |
| Justfile | <https://github.com/aaif-goose/goose/blob/main/Justfile> | `aa294302fe501b5c2bf6a196288530c3384f6b26` |

### Patterns reused

- Separate CI from package builds and from stable release publication.
- Use `workflow_call` for reusable platform build logic where it reduces
  duplication.
- Build on native runner architectures and pass an explicit Rust target triple.
- Upload immutable workflow artifacts with checksums.
- Trigger stable releases from tags and keep ordinary branch pushes out of the
  stable release path.
- Use concurrency groups, least-privilege `permissions`, and optional macOS
  signing/notarization.
- Validate packages before publishing them.

### Patterns deliberately not copied

- Goose's Electron desktop bundling, `app-update.yml`, DMG installer, and
  auto-update metadata do not apply to Gaggle's server-served UI.
- Goose's `release/*` branch model and `v1.*` tag pattern are not Gaggle
  conventions. Gaggle uses `development`/`main` and a workspace version tag
  such as `v0.1.0`.
- Goose's canary GitHub Release is not introduced. Development artifacts are
  uploaded as Actions artifacts and are clearly associated with the
  triggering commit.
- Goose's npm/pnpm, Hermit, Windows, and ACP package jobs are not relevant to
  this repository.
- Goose's Linux package formats are not assumed to work. Gaggle starts with a
  portable `.tar.gz`, which can be validated without a GUI runtime.

## 5. Comparison report

Status values: `Implemented`, `Partial`, `Missing`, `Needs investigation`.

| Capability | Goose implementation | Current Gaggle implementation | Gap | Proposed action |
| :--- | :--- | :--- | :--- | :--- |
| CI on `development` | Goose CI targets `main`; other workflows target release branches | Existing CI pushes/PRs on `development` and `main` | `Partial` | Keep triggers, add manual and merge-queue triggers, concurrency, and explicit compilation check |
| CI on `main` | CI triggers on `main` | Existing CI triggers on `main` | `Implemented` | Preserve required check names |
| Pull request checks | CI on PRs | Existing CI on PRs to both branches | `Implemented` | Add concurrency and UI syntax validation |
| Rust formatting and linting | `cargo fmt`, Clippy in CI | Existing `cargo fmt`, `cargo clippy -D warnings` | `Implemented` | Add `--locked`; keep `-D warnings` |
| Unit and integration tests | `cargo test` | Existing `cargo test --workspace` on Ubuntu/macOS | `Implemented` | Add `--locked` |
| Cross-platform builds | Goose builds native Linux/macOS packages | No package build workflow | `Missing` | Add native macOS arm64/x64 and Linux x86_64/aarch64 package matrix |
| macOS application ZIP | Goose packages an Electron `Goose.app` ZIP | No macOS bundle | `Missing` | Add `Gaggle.app` assembly, ZIP, and bundle validation |
| Linux packages | Goose builds CLI/desktop packages across Linux targets | No Linux package | `Missing` | Add portable `Gaggle-linux-<arch>.tar.gz` with user install script and desktop entry |
| Artifact validation | Goose validates app resources and updater files | No artifact validation | `Missing` | Validate extracted bundles, architectures, dependencies, checksums, and server startup |
| Canary/development builds | Goose publishes a canary release from `main` | No canary or branch artifacts | `Missing` | Upload commit-scoped Actions artifacts for `development` and `main`; do not create a stable release |
| Tagged stable releases | Goose release workflow handles tags and release assets | No release workflow | `Missing` | Add `v*` tag release workflow with main-ancestry validation and asset upload |
| Checksums and provenance | Goose publishes checksums/update metadata | No checksums | `Missing` | Add SHA-256 files, per-artifact JSON metadata, and a combined manifest |
| Code signing and notarization | Goose has optional Apple signing/notarization | No signing setup | `Missing` | Add optional secrets-gated signing/notarization; unsigned builds remain possible |
| UI/frontend checks | Goose runs pnpm/Node checks | No separate frontend build; UI is embedded Rust | `Partial` | Extract inline scripts and run `node --check` |
| Dependency vulnerability checks | Goose has additional dependency checks | Dependabot and CodeQL only | `Partial` | Document current advisory state; do not add a blocking check with known failures |
| Workflow action pinning | Goose pins many actions to SHAs | Existing workflows use major tags | `Partial` | Pin actions in the new/modified workflows; use Dependabot to review updates |
| Workflow concurrency | Goose uses concurrency groups | None | `Missing` | Add CI/package/release concurrency groups |
| Packaging documentation | Goose has build/packaging docs | README build/test only | `Partial` | Add CI/CD audit, packaging guide, artifact names, and limitations |

## 6. Current commands validated locally

These commands are supported by the repository and are used by the new
pipeline. Exact results from this work are in section 10.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
bash -n .github/scripts/*.sh
MACOSX_DEPLOYMENT_TARGET=12.0 cargo build --release --locked -p gaggle-cli --bin gaggle --bin gaggle-desktop
```

`gaggle-desktop` is a new entry point in `gaggle-cli`. It starts the existing
Axum server and opens the default browser after `/api/health` is reachable. It
does not introduce a new UI framework or a second server implementation.

## 7. Packaging design decisions

### macOS

- Artifact: `Gaggle-macOS-arm64.zip` and `Gaggle-macOS-x64.zip`.
- Bundle: `Gaggle.app` with `Contents/Info.plist`,
  `Contents/MacOS/Gaggle`, `Contents/Resources/bin/gaggle`, an icon, the logo,
  and the license.
- The CLI is kept under `Contents/Resources/bin` because macOS filesystems are
  commonly case-insensitive; a `Contents/MacOS/gaggle` CLI would collide with
  the `Gaggle` launcher.
- `CFBundleIdentifier` is
  `io.github.frengkysorituamanurung.gaggle`.
- `CFBundleExecutable` is `Gaggle`.
- The minimum system version is macOS 12.0.
- No `.dmg`, `.pkg`, auto-updater, or embedded webview is produced.
- Signing and notarization are optional and run only when all Apple secrets
  exist. Unsigned releases are labeled unsigned and are not presented as
  notarized.

### Linux

- Artifact: `Gaggle-linux-x86_64.tar.gz` and
  `Gaggle-linux-aarch64.tar.gz`.
- Contents: `bin/gaggle`, `bin/gaggle-desktop`, `install.sh`,
  `share/applications/gaggle.desktop`, a scalable icon, license, README, and
  package metadata text.
- The launcher opens the user's browser; the UI remains embedded in the server.
- `install.sh` installs under `~/.local` and does not require `sudo`.
- No `.deb`, `.rpm`, AppImage, or bundled web runtime is advertised until it
  can be built and validated.

### Artifact metadata

Each package is accompanied by:

- `<artifact>.sha256` with the checksum of the final archive.
- `<artifact-name>.json` with version, commit, ref, OS, architecture, target,
  format, checksum, signing state, and workflow run URL.
- A workflow-level `SHA256SUMS` and `build-manifest.json` for ordinary package
  builds. Manifest generation rejects an incomplete package set.

## 8. Release policy

- Pull requests and ordinary pushes to `development` and `main` never create a
  stable GitHub Release.
- Pull requests targeting `development` or `main` run the package matrix and
  upload commit-scoped workflow artifacts. They do not receive signing or
  release secrets.
- Pull request, `development`, and `main` package artifacts are workflow
  artifacts, retained for 14 days, named with the full commit SHA.
- Stable releases are created only by pushing a tag matching `v<workspace
  version>` (currently `v0.1.0`) whose commit is already on `main`.
- The release workflow verifies that:
  1. the tag matches `Cargo.toml`;
  2. the tag points at the checked-out commit;
  3. the commit is an ancestor of `origin/main`;
  4. the working tree is clean.
- The tagged commit runs formatting, Clippy, compilation, tests, the
  embedded-UI check, and shell-syntax validation before platform builds begin.
- All platform builds must succeed before the release job publishes assets.
- The release job has `contents: write`; build jobs keep `contents: read`.
- The `release` environment can be protected by repository reviewers even
  though the workflow does not require it.
- Release assets are uploaded only after checksum validation and manifest
  generation.

## 9. Security and dependency policy

Existing controls:

- CodeQL Rust analysis for pushes, PRs, and a weekly schedule.
- Dependabot for Cargo and GitHub Actions updates.
- Branch cleanup is guarded and only has repository write permission for
  deleting ephemeral branches.
- No `pull_request_target` workflow executes untrusted code.
- No signing or provider credentials are exposed to pull requests.

Current advisory evidence (checked locally during this audit):

- `cargo audit` reports one vulnerability: `ctutils 0.4.3`
  (`RUSTSEC-2026-0385`, fixed in `>=0.4.4`).
- `cargo audit` reports one unmaintained dependency: `unsafe-libyaml 0.2.11`
  (`RUSTSEC-2024-0449`) through `serde_yaml_ng` in `gaggle-recipes`.
- `cargo deny check` has no repository configuration. Its default policy
  rejects common `MIT OR Apache-2.0` licenses and also fails on the
  unmaintained advisory.

Decision: do not add a blocking `cargo audit` or `cargo deny` job until the
known findings are triaged. A permanently failing required check would hide
new regressions. The follow-up should either update the affected dependency or
add a reviewed `deny.toml` with an explicit, documented exception policy. CodeQL
and Dependabot remain active in the meantime.

## 10. Verification status

Items below distinguish local verification from GitHub Actions verification.

| Check | Status |
| :--- | :--- |
| Workflow YAML syntax (`actionlint`) | Executed locally; see final task report |
| Shell script syntax (`bash -n`) and ShellCheck | Executed locally |
| Embedded UI JavaScript (`node --check`) | Executed locally |
| `cargo fmt --all -- --check` | Executed locally |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Executed locally |
| `cargo check --workspace --all-targets --locked` | Executed locally |
| `cargo test --workspace --locked` | Executed locally: 64 tests passed |
| macOS arm64 bundle assembly and ZIP validation | Executed locally on macOS arm64 |
| macOS x64 bundle assembly and ZIP validation | Executed locally by cross-building the x86_64 target; extracted-binary smoke test passed |
| Packaged server health check | Executed locally against both extracted macOS apps |
| macOS LaunchServices launch | Executed locally with `open --args --no-browser`; `/api/health` responded |
| Linux package build and validation | Not executed locally (Docker daemon unavailable and no Linux host); configured for `ubuntu-22.04` and `ubuntu-22.04-arm` |
| GitHub Actions CI/package/release execution | Not executed; no commit or push was requested or performed |
| Apple signing/notarization | Not executed; no Apple credentials are available |
| Linux GUI launch | Not executed; the app is a browser-launched local server, not a native GUI |

## 11. Assumptions and residual risks

- `ubuntu-22.04` and `ubuntu-22.04-arm` are GitHub-hosted runner labels used
  to build against a glibc 2.35 baseline. GitHub's runner-images documentation
  lists both labels; the first Actions run must still confirm availability.
- `macos-15-intel` is used for the Intel macOS build. If the runner label is
  unavailable or retired, the x64 job should be disabled rather than
  cross-compiled without validation.
- `MACOSX_DEPLOYMENT_TARGET=12.0` is set for package builds so the binary
  matches the `LSMinimumSystemVersion` value.
- Unsigned macOS artifacts may require Gatekeeper confirmation. The project
  must not instruct users to disable Gatekeeper.
- The macOS app is a local server launcher, not a native webview. The browser
  opening step is best-effort; users can open `http://127.0.0.1:4321`.
- Linux builds are portable archives, not distribution packages. Users need a
  compatible glibc-based distribution and `xdg-open` for automatic browser
  startup.
- The macOS bundle was launched locally through LaunchServices with
  `--args --no-browser`, and both architectures passed an extracted-binary
  smoke test. CI does not automate Finder GUI interaction.

## 12. Files introduced or changed

| Path | Purpose |
| :--- | :--- |
| `.github/workflows/ci.yml` | Required formatting, linting, compilation, tests, UI script validation; preserves branch-protection check names |
| `.github/workflows/reusable-build.yml` | Reusable native package build, validation, checksum, metadata, and artifact upload |
| `.github/workflows/package.yml` | Development/main package matrix and combined manifest |
| `.github/workflows/release.yml` | Tagged stable release validation, package builds, and asset publication |
| `.github/workflows/codeql.yml` | Pinned CodeQL v4 action, concurrency group, and locked build |
| `.github/workflows/pr-summary.yml` | Pinned checkout action and bounded job timeout |
| `.github/workflows/branch-cleanup.yml` | Bounded cleanup job timeout |
| `.github/PULL_REQUEST_TEMPLATE.md` | Packaging and CI verification reminders |
| `.github/scripts/package-macos.sh` | Build `Gaggle.app` and the macOS ZIP |
| `.github/scripts/validate-macos-bundle.sh` | Validate extracted macOS bundle, architecture, minimum OS, dependencies, signature state, and startup |
| `.github/scripts/package-linux.sh` | Build the portable Linux archive and user installer |
| `.github/scripts/validate-linux-package.sh` | Validate extracted Linux archive, installer syntax, permissions, architecture, shared libraries, and startup |
| `.github/scripts/sign-and-notarize-macos.sh` | Optional Developer ID signing and notarization |
| `.github/scripts/smoke-test-server.sh` | Start the packaged launcher and verify `/api/health` |
| `.github/scripts/write-artifact-metadata.sh` | Generate SHA-256 and per-artifact JSON metadata |
| `.github/scripts/generate-build-manifest.sh` | Require the expected package set, verify checksums, and generate `SHA256SUMS`/`build-manifest.json` |
| `.github/scripts/generate-release-notes.sh` | Generate release notes from Git history |
| `.github/scripts/validate-release.sh` | Enforce tag/version/main ancestry rules |
| `.github/scripts/check-ui-embedded.sh` | Parse embedded UI scripts with Node |
| `.github/scripts/generate-macos-icon.sh` | Regenerate `assets/Gaggle.icns` from the SVG logo |
| `crates/gaggle-cli/src/bin/gaggle-desktop.rs` | Native desktop launcher that starts the server and opens the browser |
| `crates/gaggle-cli/Cargo.toml` | Registers the desktop binary |
| `assets/Gaggle.icns` | macOS application icon generated from the project logo |
| `rust-toolchain.toml` | Stable toolchain plus rustfmt/Clippy components |
| `.gitignore` | Ignore generated `dist/` artifacts |
| `docs/development/packaging.md` | User and maintainer packaging/installation guide |
| `README.md` | Discovery links for desktop packages and packaging docs |
| `CONTRIBUTING.md` | Packaging/CI verification guidance for contributors |
