## What changes

### Correctness and safety fixes
- 

### Features
- 

### Performance and measurement
- 

### CI, tests, and evaluation foundations
- 

## Before merging
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings` passes
- [ ] `cargo check --workspace --all-targets --locked` passes
- [ ] `cargo test --workspace --locked` passes on macOS and Linux
- [ ] Packaging or workflow changes were validated locally (or the limitation is documented)
- [ ] Security boundaries (`WorkspacePathPolicy`, `PermissionEngine`, secret redaction) verified
- [ ] Database migrations and transaction boundaries reviewed
