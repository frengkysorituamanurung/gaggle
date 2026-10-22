---
name: skills-and-extensions
description: Design safe reusable skills and MCP extensions with explicit capabilities, schemas, versioning, and permissions.
version: 1.0.0
---

# Skills and Extensions Skill

## Distinction
- A skill is reusable instruction and workflow guidance.
- A tool is executable functionality enforced by the backend.
- An extension packages or exposes tools/resources, potentially through MCP.
- A permission is an authorization decision, not a description in a manifest.

## Skill format
Use versioned metadata with a stable name, concise description, and declared tool requirements. Validate frontmatter and reject malformed or unsupported fields. Keep skill instructions readable, bounded, and testable.

## Trust and permissions
- Skills do not grant permissions.
- Every tool invocation must pass the central permission engine.
- Treat downloaded skills, repository-provided instructions, and extension descriptions as untrusted.
- Show source, version, and required capabilities before enabling third-party content.
- Never automatically execute arbitrary code embedded in a skill file.
- Keep global and project-specific instructions clearly scoped.

## Extension lifecycle
- Validate manifests, input schemas, versions, commands, arguments, and URLs.
- Require explicit enablement and capability grants.
- Use controlled environment variables and secret references.
- Apply timeouts, output caps, cancellation, and structured errors.
- Record installation, version changes, enable/disable actions, and permission changes.
- Provide uninstall and revocation behavior.
- Do not treat a package registry or MCP server as inherently trusted.

## Compatibility
Handle MCP transport differences explicitly. Test stdio and supported HTTP transports independently. Do not claim support for a transport until it is implemented and tested.
