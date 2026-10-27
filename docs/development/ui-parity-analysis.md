# Gaggle vs Goose UI/UX Parity Analysis

Status: analysis / roadmap. No code changes in this document.

## 1. Executive summary

Gaggle's TypeScript UI (`ui/`) is currently a scaffold: it reuses Goose's visual
tokens and component names, but most screens and components are stubs or minimal
implementations. Goose's desktop UI (`goose/ui/desktop`) is the reference product
and is roughly 6x larger (502 vs 87 source files). The primary gap is not
styling but functional depth: state management, streaming chat, tool approval
cards, provider/model flows, settings sections, and backend wiring.

Visual tokens and Tailwind setup are already aligned with Goose
(`ui/src/styles.css` mirrors `goose/ui/desktop/src/styles/main.css`). Therefore the
remaining work is predominantly component implementation and API adaptation.

## 2. Size and surface comparison

| Area | Goose (`ui/desktop/src`) | Gaggle (`ui/src`) |
|---|---:|---:|
| Source files (.ts/.tsx/.css) | 502 | 87 |
| Main stylesheet | 991 lines | 1162 lines |
| App shell | full nested routes, providers | flat routes, minimal providers |
| Chat | BaseChat 631 lines + ChatInput 1963 lines | ChatScreen 305 lines |
| Settings | large SettingsView + sections | SettingsScreen 213 lines |

Gaggle components with fewer than 20 lines are effectively stubs. Examples:

- components/ToolApprovalButtons.tsx (9 lines)
- components/ToolCallCard.tsx (22 lines but minimal)
- components/ToolCallConfirmation.tsx (10 lines)
- components/MentionPopover.tsx (4 lines)
- components/MessageQueue.tsx (4 lines)
- components/LauncherView.tsx (21 lines)
- screens/ConfigureProvidersScreen.tsx (4 lines)
- screens/SkillsScreen.tsx (28 lines)
- screens/RecipesScreen.tsx (29 lines)

## 3. Architecture differences

### Goose

- Desktop renderer built with React + TypeScript.
- Data layer is ACP (Agent Client Protocol) over HTTP/WebSocket plus
  Electron IPC (`window.electron.*`).
- Heavy use of contexts: ChatContext, ThemeContext, FeaturesContext,
  ModelAndProviderContext, ConfigContext.
- Session-centric: ChatProvider, BaseChat, useChatSession, useAutoSubmit.
- Rich component library for messages, tool calls, approvals, MCP apps, recipes,
  schedules, providers, settings.

### Gaggle

- Web frontend served by gaggle-server (Axum) from ui/dist.
- Data layer is a small REST/SSE client in ui/src/lib/api.ts.
- State is a minimal AppProvider in ui/src/lib/store.tsx.
- Backend exposes /api/* routes and /api/runs/{id}/events SSE.
- Agent execution is server-side (RunEngine); UI must poll/SSE stream run events.

Implication: Gaggle cannot copy Goose components verbatim. Every Goose data call
must be mapped to Gaggle's REST/SSE API or implemented as new backend work.

## 4. Visual parity status

Already aligned:

- Tailwind v4 setup and tw-animate-css.
- Goose semantic color tokens (background-*, text-*, border-*).
- Dark/light theme variables.
- Typography and animation utilities.
- Layout primitives (AppLayout, NavigationPanel, NavigationContext).

Needs work:

- Component density, spacing, hover/focus states.
- Empty/loading/error states.
- Responsive behavior and panel resizing.
- Accessibility: keyboard navigation, focus restoration, reduced motion.
- Iconography and exact control shapes.

## 5. Functional gap inventory

### A. Chat experience (highest priority)

Goose has:

- BaseChat, MessageList, ChatInput (1963 lines), streaming markdown, tool
  call rendering, approvals, file drop, quote, search, live voice.

Gaggle has:

- Basic ChatScreen with project/chat selection, manual SSE streaming text
  accumulation, and a simple textarea.
- No full message part rendering parity.
- No robust tool approval card flow.
- No file attachments, quote, search-in-conversation, or live voice integration.

### B. Navigation and layout

Gaggle navigation is functional but simpler. Missing:

- Session grouping and recent chats behavior matching Goose.
- Collapsible sidebar animations and resize persistence.
- Titlebar drag region and window controls for desktop wrapper.
- Mobile/narrow layout behavior.

### C. Hub / home

Gaggle HubScreen is a basic greeting + project selector. Goose Hub is richer:
recent sessions, recipes, announcements, launcher entry points.

### D. Providers and models

Gaggle API supports provider/model CRUD, but UI only lists providers/models in
chat. Missing:

- Provider settings page.
- OAuth flows (ChatGPT) UI.
- Model picker with capabilities and refresh.
- Provider configuration forms comparable to Goose.

### E. Settings

Gaggle SettingsScreen is 213 lines and likely incomplete. Goose has a large
settings system with sections for app, chat, models, providers, auth, permissions,
agent loop, local inference, dictation, live voice, keyboard, response styles,
security, extensions, reset provider, goosehints, prompts.

### F. Approvals and permissions

Gaggle has backend approvals and permission modes, but UI approval cards are stubs.
Goose has rich approval cards with remember rules and impact explanations.

### G. Workspace, files, git, worktrees

Gaggle backend exposes project files, git status, diff, worktrees, merge/discard.
UI has WorkspaceScreen (99 lines) and no full diff review panel parity.

### H. Extensions, MCP, apps

Gaggle backend has extension inventory and tool discovery. UI has ExtensionsScreen
(62 lines) and apps views that are stubs. Goose has full extension management,
MCP apps, pip windows, app views.

### I. Recipes, schedules, skills, memories

Backend exists; UI screens are stubs (28-94 lines). Need full CRUD and execution
flows.

### J. Terminal

Backend terminal routes exist; TerminalScreen is 64 lines. Need PTY UI parity.

### K. Onboarding and diagnostics

Gaggle has OnboardingGuard, consent prompt, announcement modal, but many onboarding
components are stubs. Diagnostics endpoint exists; UI surface is minimal.

## 6. API mapping (Goose concept -> Gaggle endpoint)

| Goose capability | Gaggle implementation | Status |
|---|---|---|
| Session list/create/load | /api/projects, /api/projects/{id}/chats, /api/chats/{id} | Partial |
| Send prompt / run | POST /api/chats/{id}/runs, SSE /api/runs/{id}/events | Partial |
| Cancel run | POST /api/runs/{id}/cancel | Wired |
| Approvals | /api/approvals/pending, /api/approvals/{id}/decide | Partial |
| Providers/models | /api/providers, /api/providers/{id}/models | Partial |
| Settings | /api/settings | Partial |
| Memories | /api/memories | Partial |
| Recipes | /api/recipes | Partial |
| Schedules | /api/schedules | Partial |
| Extensions | /api/extensions | Partial |
| Terminal | terminal routes | Partial |
| Worktrees/diff | /api/projects/{id}/diff, /api/worktrees/{id}/* | Partial |
| Compaction | /api/chats/{id}/compact | Partial |

## 7. Recommended implementation phases

### Phase 0 - Stabilize adapter boundary

- Keep ui/src/lib/api.ts as the single backend client.
- Expand types in ui/src/types.ts to cover all endpoints.
- Add a small adapter for run events with typed SSE parsing.
- Add UI unit tests for adapter (Vitest).

### Phase 1 - Chat parity (core)

- Rebuild ChatScreen around a MessageList + ChatInput component.
- Render message parts (text, tool call, tool result, thinking, approval).
- Implement robust SSE handling: delta, tool call, approval, error, done.
- Add approval cards with remember-rule options.
- Add stop/cancel control and run status.
- Persist scroll position and draft.

### Phase 2 - Navigation and global UX

- Match Goose sidebar behavior exactly.
- Persist sidebar width and collapse state.
- Add global search/command palette if needed.
- Add toast/error handling parity.

### Phase 3 - Providers, models, settings

- Implement provider settings pages.
- Implement model picker and refresh.
- Implement full settings sections (at least functional ones backed by /api/settings).
- Add onboarding flows.

### Phase 4 - Workspace, files, git, terminal

- File browser, diff viewer, worktree merge/discard.
- Terminal PTY UI.

### Phase 5 - Extensions, apps, recipes, schedules, memories, skills

- Full CRUD and execution UIs.

### Phase 6 - Accessibility, responsiveness, polish

- Keyboard-only flows, focus restoration, reduced motion.
- Cross-platform verification.

## 8. Risks and constraints

- Large surface area; full parity is a multi-week effort.
- Goose is ACP/Electron-based; Gaggle is REST/SSE/web; adapter complexity.
- Many Gaggle backend features exist but UI stubs reduce perceived parity.
- Packaging must include rebuilt ui/dist.
- No Windows support in scope.

## 9. Immediate next actions

1. Implement Phase 0 adapter and types.
2. Rebuild chat experience (Phase 1) first, because it is the primary user journey.
3. Only after chat is stable, expand settings and workspace.
4. Track parity with a checklist per Goose screen/component.

## 10. Suggested parity checklist (initial)

- [ ] Chat streaming + tool calls + approvals
- [ ] Sidebar navigation exact behavior
- [ ] Hub recent sessions
- [ ] Provider/model management
- [ ] Settings sections
- [ ] Workspace/diff review
- [ ] Terminal
- [ ] Extensions/apps
- [ ] Recipes/schedules/skills/memories
- [ ] Onboarding/diagnostics

This document is an analysis only. Implementation should proceed in phases, starting with chat parity.
