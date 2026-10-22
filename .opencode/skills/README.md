# Gaggle Skills Pack

Reusable AI-agent skill definitions for planning, designing, engineering, securing, testing, and maintaining Gaggle.

## Included skills

- `product-manager/SKILL.md` — product planning and prioritization
- `ui-ux/SKILL.md` — UX, accessibility, and cross-platform interactions
- `rust-engineering/SKILL.md` — idiomatic Rust engineering
- `architecture/SKILL.md` — system architecture and ADRs
- `security/SKILL.md` — threat modeling and security controls
- `agent-runtime/SKILL.md` — agent loop, tools, context, and recovery
- `database-sqlite/SKILL.md` — SQLite and SQLx
- `provider-integrations/SKILL.md` — model providers
- `testing-qa/SKILL.md` — quality and regression testing
- `observability-reliability/SKILL.md` — diagnostics and reliability
- `skills-and-extensions/SKILL.md` — skill and MCP extension safety
- `git-workflows/SKILL.md` — Git safety and diff review
- `performance-cross-platform/SKILL.md` — performance and platform support
- `documentation-open-source/SKILL.md` — docs and contributor workflow
- `research-thinking/SKILL.md` — evidence-based decisions and uncertainty

## Integration

These files are framework-neutral Markdown skill definitions. Adapt frontmatter and directory placement to the AI coding agent or skill loader used by your repository. A skill is guidance, not a security boundary: all tool permissions and safety checks must be enforced by Gaggle's backend.

Suggested repository layout:

```text
.gaggle/
  skills/
    product-manager/SKILL.md
    ui-ux/SKILL.md
    rust-engineering/SKILL.md
    architecture/SKILL.md
    security/SKILL.md
    agent-runtime/SKILL.md
    database-sqlite/SKILL.md
    provider-integrations/SKILL.md
    testing-qa/SKILL.md
    observability-reliability/SKILL.md
    skills-and-extensions/SKILL.md
    git-workflows/SKILL.md
    performance-cross-platform/SKILL.md
    documentation-open-source/SKILL.md
    research-thinking/SKILL.md
```

## Important security note

Do not auto-execute code from skill files. Validate skill metadata, display provenance for third-party skills, and route every tool call through the central permission engine.
