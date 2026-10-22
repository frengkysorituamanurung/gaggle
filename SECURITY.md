# Security Policy

Security is a foundational priority for **Gaggle**. Because Gaggle is a self-hosted AI agent that interacts directly with local filesystems, PTY terminals, sandboxed browsers, and Model Context Protocol (MCP) extensions on the host machine, we treat any vulnerability affecting permission enforcement, command execution, or credential handling with the highest severity.

---

## Supported Versions

Gaggle is currently under active development (`0.1.x`). Security patches are provided for the latest release and the `main` branch.

| Version | Supported |
| :--- | :--- |
| `0.1.x` (latest) | :white_check_mark: Yes |
| `< 0.1.0` | :x: No |

---

## Security Architecture & Boundaries

Contributors and auditors should be aware of the core security invariants enforced across the Gaggle workspace:

1. **Zero Plaintext Secrets in SQLite**
   - Provider API keys, OAuth tokens, and extension environment secrets (`env_secret_refs`) must **never** be stored in plaintext inside `gaggle.db`.
   - The database only stores a `secret_ref` key referencing the host OS Keychain (macOS Keychain, Linux Secret Service / KWallet, or Windows Credential Manager).

2. **7-Step Permission Engine & Command Rules**
   - New chats default to the `ask` permission mode (manual approval required for mutating tools), not autonomous `full` mode.
   - Shell command rules (`command_rules`) with `action = 'deny'` are **absolute** and must block execution even when a chat is configured in `full` mode.
   - The `chat_only` permission mode must reject every tool call unconditionally.

3. **Workspace & Git Worktree Isolation**
   - In `readonly` mode, the agent must not modify any files inside the project's `root_path`.
   - In `worktree` mode, all file mutations and shell executions must remain confined to the isolated Git worktree directory associated with that chat.

4. **Code Mode & Extension Sandboxing**
   - Scripts executed inside Code Mode (`code_executions`) are constrained by strict memory limits (`memory_limit_mb`) and execution timeouts (`timeout_ms`).
   - Extensions (`extensions`) may only exercise capabilities (`fs.read`, `fs.write`, `shell`, `net`, `browser`, `memory`) that have been explicitly granted in `extension_permissions`.

5. **Device Pairing & Token Hashing**
   - Access tokens for paired remote devices (`devices`, such as mobile browsers connecting to a local server) are stored exclusively as cryptographic hashes (`token_hash`) and can be revoked immediately via `revoked_at`.

---

## Reporting a Vulnerability

**Please DO NOT report security vulnerabilities through public GitHub issues, discussions, or pull requests.**

If you discover a potential security vulnerability in Gaggle (such as a permission engine bypass, command injection, path traversal outside a project workspace, or credential exposure):

1. **Private Reporting Channel**:
   - Report the issue privately via **[GitHub Private Vulnerability Reporting / Security Advisories](https://github.com/frengky/gaggle/security/advisories/new)** on this repository, or contact the project maintainers directly.

2. **What to Include**:
   - A clear description of the vulnerability and the affected crate(s) (`gaggle-core`, `gaggle-db`, `gaggle-tools`, `gaggle-mcp`, `gaggle-server`, etc.).
   - Step-by-step reproduction instructions or a minimal Proof of Concept (PoC).
   - Potential security impact (e.g., unauthorized tool execution in `ask` mode).
   - Suggested remediation or mitigation, if available.

3. **Response Timeline & Coordinated Disclosure**:
   - **Acknowledgment**: Within **48 hours** of receiving your report.
   - **Triage & Assessment**: Within **5 business days**, we will confirm the vulnerability, assess its severity, and share a remediation plan.
   - **Fix & Disclosure**: Once a patch is tested and released, we will publish a security advisory and credit the reporter (unless you prefer to remain anonymous).
