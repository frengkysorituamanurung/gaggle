---
name: ui-ux
description: Design and implement clear, accessible, keyboard-first Gaggle experiences for macOS and Linux.
version: 1.0.0
---

# UI/UX Skill

## Mission
Make Gaggle feel like a focused developer workspace: simple for first-time users, efficient for experts, and transparent during agent execution.

## Core principles
- Show what the agent is doing, what changed, and what requires user action.
- Keep the conversation usable during long-running work.
- Use progressive disclosure for advanced settings.
- Prefer predictable, low-noise interactions over decorative animation.
- Preserve user input and context during navigation, errors, and reconnects.
- Use consistent design tokens for spacing, typography, color, borders, and states.

## Main information architecture
- Projects and recent chats.
- Conversation and task progress.
- Contextual Changes/diff inspector.
- Tool activity and approval queue.
- Provider/model and permission controls.
- Settings, diagnostics, and later-phase skills/extensions.

## Required states
Design loading, empty, success, partial success, waiting for approval, running, paused, cancelled, failed, offline/reconnecting, and recovery states. Never imply that a task or test succeeded when it did not.

## Interaction requirements
- Stream content without freezing the UI.
- Keep run state in the backend; UI component lifecycle must not own long-running tasks.
- Provide obvious Stop/Cancel controls while a run is active.
- Explain approval requests with the exact operation, target, working directory, and expected impact.
- Show tool calls, outputs, and errors distinctly from assistant prose.
- Make diff review accessible without losing conversation context.
- Use confirmation for irreversible actions and make destructive consequences explicit.
- Avoid modal dialogs when an inline contextual action is sufficient.
- Provide empty-state guidance and actionable error messages.

## Accessibility
- Support keyboard navigation and visible focus.
- Do not rely on color alone to communicate state.
- Maintain readable contrast and sensible text scaling.
- Respect reduced-motion settings.
- Label controls for assistive technologies.
- Check tab order, focus restoration, and screen-reader announcements for streaming and approvals.

## Cross-platform UX
- Verify keyboard shortcuts on macOS and Linux.
- Use native file selection and platform conventions where practical.
- Handle narrow windows and resizable side panels.
- Do not assume identical font metrics, WebView behavior, filesystem paths, or system theme behavior.

## Workflow
1. Inspect existing components and visual conventions.
2. Map the user journey and identify friction.
3. Define interaction states and acceptance criteria before implementation.
4. Reuse components and design tokens.
5. Implement responsive, accessible behavior.
6. Test keyboard-only usage, error states, long-running tasks, and both supported platforms.
7. Report limitations and attach real screenshots or test evidence when available.

## Do not
- Invent controls that do nothing.
- Hide failures or approval states.
- Clear unsent user input after transient errors.
- Use excessive animations, nested dialogs, or unexplained technical jargon.
