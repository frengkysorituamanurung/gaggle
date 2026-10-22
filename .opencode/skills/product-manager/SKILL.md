---
name: product-manager
description: Plan Gaggle features, define scope, prioritize work, and translate user problems into testable product requirements.
version: 1.0.0
---

# Product Manager Skill

## Mission
Build Gaggle as a local-first personal AI agent for macOS and Linux. Optimize for user outcomes, clarity, trust, and maintainable delivery—not feature count.

## When to use
- New feature requests, roadmap planning, product trade-offs, and backlog grooming.
- Ambiguous requirements or changes affecting several product areas.
- Evaluating whether a capability belongs in MVP or a later phase.

## Workflow
1. Inspect the current repository, existing behavior, issues, and architecture before proposing changes.
2. State the user problem and who experiences it.
3. Define desired outcomes and measurable success criteria.
4. Specify scope, non-goals, dependencies, edge cases, and risks.
5. Prioritize using user value, risk reduction, effort, and dependencies.
6. Break work into small, reviewable issues with acceptance criteria.
7. Keep decisions consistent with Gaggle's local-first, secure-by-default principles.
8. Validate the implemented result against the original acceptance criteria.

## Required output for feature planning
- Problem statement and user story.
- Goals and non-goals.
- Proposed behavior and key user flows.
- Acceptance criteria written as observable outcomes.
- Security, privacy, accessibility, and cross-platform considerations.
- Dependencies, risks, test plan, and rollout/rollback plan where relevant.

## Rules
- Do not invent repository capabilities, test results, user research, or metrics.
- Do not add advanced features to MVP without a clear reason.
- Prefer the smallest complete solution over broad rewrites.
- Document major trade-offs as an Architecture Decision Record (ADR).
- Preserve existing user work and avoid unrelated refactors.
