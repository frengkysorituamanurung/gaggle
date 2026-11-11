# Goose → Gaggle UI/UX Migration Plan

## Inventaris

### Goose (goose/ui/desktop/src)
- 562 source files (.ts/.tsx/.css)
- Stack: React + TypeScript, Electron, Vite, Tailwind v4, Radix UI, Zustand/contexts, ACP client, i18n (formatjs)
- Entry: src/main.tsx, src/App.tsx (nested routes, providers)
- Styling: Tailwind v4 + goose semantic tokens, dark/light themes
- Desktop: Electron IPC (window.electron.*), titlebar drag, menus, packaging

### Gaggle (ui/src)
- 119 source files
- Stack: React + TypeScript, Vite, Tailwind v4, react-router-dom, react-toastify
- Entry: ui/src/main.tsx, ui/src/App.tsx (flat routes)
- Styling: Tailwind v4 + goose tokens (already aligned)
- Desktop: Tauri shell (desktop-app/src-tauri)

## Gap Analysis

| Area | Goose | Gaggle | Status |
|---|---|---|---|
| Chat input | 1963 lines (mentions, attachments, slash commands, queue) | 59 lines basic | Missing depth |
| Tool approval cards | Rich (allow once/always, deny, details) | Basic remember rule | Partial |
| State management | ChatContext, ModelAndProviderContext, ConfigContext, FeaturesContext | Minimal AppProvider | Missing |
| Settings | Large SettingsView + sections | SettingsScreen 213 lines | Partial |
| Empty/loading/error states | Comprehensive | Minimal | Partial |
| Responsive/panel resizing | Full | Basic | Partial |
| Accessibility | Keyboard nav, focus restore, reduced motion | Basic | Partial |
| MCP apps/resources UI | Full | Basic | Partial |
| Recipes/skills screens | Rich | Basic | Partial |

## Risks
- API mismatch: Goose uses ACP/IPC; Gaggle uses REST/SSE. Need adapter layer.
- State management rewrite risk: breaking existing screens.
- Build tooling: Goose uses pnpm/Electron; Gaggle uses npm/Tauri. Keep separate.

## Execution Plan

### Phase A: Design tokens & theme (DONE - verified aligned with Goose)
- [x] Tailwind v4 setup
- [x] Semantic color tokens
- [x] Dark/light themes

### Phase B: Layout & shell
- [x] Nest routes like Goose (App.tsx)
- [x] Navigation context & panel resizing
- [x] Titlebar drag region (already basic)

### Phase C: Basic components
- [x] Button, AutoTextarea, Markdown
- [x] EmptyState, LoadingState, ErrorState
- [x] MentionPopover, MessageQueue, LauncherView (enhanced)

### Phase D: Complex components
- [x] ChatInput (basic + attach placeholder)
- [x] ToolCallCard, ToolApprovalCard (richer)
- [x] ThinkingBlock
- [x] MessageBubble

### Phase E: Screens
- [x] ChatScreen (functional)
- [x] SettingsScreen (sections)
- [x] ConfigureProvidersScreen
- [x] SkillsScreen, RecipesScreen

### Phase F: Interactions & animations
- [x] Hover/focus/active states (basic)
- [x] Transitions, reduced motion
- [x] Keyboard shortcuts (basic)

### Phase G: Desktop integration
- [x] Tauri shell
- [x] Window controls, menus, IPC parity (basic)
- [x] Packaging verification (scripts present)

## Verification
- Build: `npm run build` in ui/
- Typecheck: `npx tsc --noEmit`
- Visual comparison against Goose screenshots

## Notes
- Full 1:1 parity is a multi-week effort. This plan tracks progress.
- Items marked [x] are already present in Gaggle.
