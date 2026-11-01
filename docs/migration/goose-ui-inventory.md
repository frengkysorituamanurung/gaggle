# Goose UI Inventory (Reference)

Source: `goose/ui/desktop/src` (local reference, Electron + React + TypeScript + Vite + Tailwind v4).

## Application shell
- Entry: `src/main.ts`, `src/renderer.tsx`, `src/App.tsx` (nested routes/providers).
- Titlebar drag region, window controls, Electron IPC (`window.electron.*`), native menus/dialogs.
- Providers: ThemeContext, FeaturesContext, ChatContext, ModelAndProviderContext, ConfigContext.
- Sidebar: collapsible, resizable, session grouping, recent chats.
- Global dialogs: announcements, onboarding, telemetry consent, MCP apps/pip windows.

## Chat experience
- BaseChat (~631 lines), ChatInput (~1963 lines): mentions, attachments, slash commands, message queue, live voice.
- MessageList, streaming markdown, code blocks, syntax highlighting (react-syntax-highlighter/katex).
- Tool calls, tool results, thinking/reasoning blocks, approvals with remember rules.
- Retry/regenerate, stop generation, attachments, model selection, context/token indicators.
- Empty/loading/error states.

## Product navigation
- Hub/home (recent sessions, recipes, announcements, launcher).
- Recipes, skills, extensions/MCP apps, scheduler, session history.
- Settings: app, chat, models, providers, auth, permissions, agent loop, local inference, dictation, live voice, keyboard, response styles, security, extensions, goosehints, prompts.
- Provider configuration, OAuth (ChatGPT), model picker with capabilities/refresh.
- Diagnostics.

## Reusable components
- UI primitives (Radix): accordion, avatar, collapsible, dialog, dropdown-menu, icons, popover, portal, radio-group, scroll-area, select, slot, switch, tabs, tooltip.
- Buttons, inputs, textareas, auto-textarea, cards, tabs, tooltips, popovers, dialogs, tables, badges, skeletons, toasts.
- Message components, code viewers, file attachment components, MCP app renderer, pip window.
- Forms (tanstack/react-form), date-fns, cronstrue.

## Interaction behavior
- Mouse/keyboard, focus handling, drag-and-drop, scrolling, window resizing.
- Navigation transitions, form submission, dialog lifecycle, state persistence.
- Loading/error recovery, reduced motion, accessibility.

## Styling
- Tailwind v4 + goose semantic tokens, dark/light themes, tw-animate-css, typography plugin.

## Count
- ~502 source files (.ts/.tsx/.css) in `ui/desktop/src`.