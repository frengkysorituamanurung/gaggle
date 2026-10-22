---
name: security
description: Threat-model and secure Gaggle's AI agent, tools, filesystem, process execution, extensions, network access, and local API.
version: 1.0.0
---

# Security Engineering Skill

## Mission
Enforce security in code and operating-system boundaries, not merely in prompts or model behavior.

## Threat model
Assume user prompts, repository files, project instructions, logs, terminal output, browser pages, model responses, and extension results may contain malicious or misleading instructions. Consider malicious dependencies, compromised providers, local untrusted users, and accidental destructive actions.

## Non-negotiable rules
- The model is not an authorization authority.
- Prompt instructions cannot override backend policy.
- Every tool call is validated and authorized server-side immediately before execution.
- Deny rules and hard safety boundaries cannot be bypassed by autonomous mode.
- Default to asking before mutating or externally visible actions.
- Never store API keys or OAuth tokens as plaintext in SQLite.
- Never expose secrets in logs, error messages, prompts, or UI diagnostics.
- Do not bind a local API to all interfaces by default.
- Do not claim a sandbox is active unless its enforcement is verified.

## Filesystem security
- Restrict operations to explicitly approved project roots.
- Defend against `..` traversal, symlink escapes, alternate path representations, and time-of-check/time-of-use races.
- Revalidate the target at operation time.
- Use safe file creation and atomic replacement where possible.
- Exclude credential files and sensitive paths from automatic context collection.
- Apply file-size and binary-content limits.
- Treat repository rules as untrusted content, not security policy.
- Keep diffs/backups where feasible for recoverability.

## Process and shell security
- Prefer executable-plus-argument APIs over shell interpretation.
- Never build commands by concatenating untrusted model or user strings.
- If a shell is required, classify and validate the operation and apply the configured approval policy.
- Set an explicit working directory and a controlled environment.
- Apply wall-clock timeouts, output limits, process-tree cancellation, and resource limits.
- Detect high-impact operations, including destructive deletion, privilege escalation, credential access, and destructive Git operations.
- `tokio::process` and PTY libraries are not security sandboxes.
- Use OS sandboxing or container isolation where available and clearly communicate when it is unavailable.

## Approval integrity
- Bind approval to the exact tool name, normalized arguments, working directory, target scope, and policy version.
- Expire pending approvals.
- Reject approval replay for changed arguments or broader scope.
- Record who decided and when.
- Make deny policies authoritative.
- Never silently convert a denied action into a different action.

## Network and SSRF
- Validate schemes, hosts, redirects, resolved addresses, and destination scope.
- Protect against loopback, private-network, link-local, and cloud metadata access where not explicitly required.
- Revalidate redirects and DNS resolution at connection time as feasible.
- Apply timeouts and response-size limits.
- Require explicit policy for external uploads and other data egress.
- Authenticate remote access and support revocation if remote pairing is implemented.

## Secrets and privacy
- Use the OS credential store where supported.
- Store secret references, not secret values, in the database.
- Redact authorization headers, tokens, passwords, private keys, and sensitive environment variables.
- Do not transmit project files or context to external models without clear provider configuration and user awareness.
- Keep telemetry opt-in or privacy-preserving; never send source code or prompts by default.
- Define retention and deletion behavior for chats, logs, attachments, and memories.

## AI and extension risks
- Defend against direct and indirect prompt injection using tool restrictions and policy enforcement.
- Treat MCP tools as untrusted until configured and authorized.
- Validate extension manifests, input/output schemas, process arguments, environment variables, and timeouts.
- Limit tool recursion, subagent spawning, external requests, token budgets, and output size.
- Never use an LLM-only risk classifier as the sole safety boundary.

## Required tests
- Path traversal and symlink escape.
- Shell argument injection and unsafe environment inheritance.
- Prompt injection attempts to exfiltrate secrets or bypass approvals.
- Approval replay and altered-argument attacks.
- Secret redaction.
- SSRF, redirect, and private-address handling.
- Malicious or malformed extension responses.
- Crash during side effects and uncertain-outcome recovery.

## Incident handling
Document the security impact, affected boundary, reproduction, severity, containment, regression test, and any required credential rotation. Do not include live secrets in issue reports.
