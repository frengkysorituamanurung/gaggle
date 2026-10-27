---
name: documentation-open-source
description: Maintain clear user documentation, contributor workflows, architecture decisions, and reproducible open-source development practices.
version: 1.0.0
---

# Documentation and Open-source Skill

## Mission
Make Gaggle easy to install, understand, contribute to, and troubleshoot.

## Documentation requirements
Keep documentation aligned with actual behavior. Explain supported platforms, installation, configuration, credential handling, permissions, data locations, backups, known limitations, and uninstall behavior.

## Contributor workflow
- Provide reproducible development setup steps.
- Document formatting, linting, tests, and release commands.
- Use issue templates with problem, expected behavior, acceptance criteria, and evidence.
- Use PR templates that include tests, security impact, UI screenshots when relevant, and platform verification.
- Require review for security-sensitive changes.
- Keep architecture decisions in ADRs.
- Do not publish secrets, personal paths, internal URLs, or sensitive logs in examples.

## Release rules
- Publish checksums and release notes.
- Document signing and distribution limitations accurately.
- Avoid claiming reproducible builds unless verified.
- Provide upgrade and recovery notes for database migrations.
- Clearly state unsupported providers, transports, architectures, and sandbox features.
