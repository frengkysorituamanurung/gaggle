# Goose → Gaggle Compatibility Matrix

| Subsystem | Goose implementation | Gaggle equivalent | Compatibility issue | Migration strategy | Verification |
|---|---|---|---|---|---|
| UI framework | React 19 + TS 5.9 + Vite 8 + Tailwind v4 + Radix | React 19 + TS 5 + Vite 6 + Tailwind v4 + Radix | Version skew (TS/Vite) | Keep Gaggle toolchain; port components, not config | tsc build |
| Desktop shell | Electron IPC (window.electron.*) | Tauri shell + Axum local server | IPC mismatch | Replace Electron calls with REST/SSE + Tauri commands | desktop startup |
| Data layer | ACP over HTTP/WS + Electron | REST/SSE under /api/* | Protocol mismatch | Adapter in ui/src/lib/api.ts + runEvents.ts | API tests |
| State | Contexts (Chat, Theme, Features, ModelAndProvider, Config) | AppProvider minimal | Missing contexts | Incrementally add contexts; keep AppProvider as root | tsc |
| Chat | BaseChat + ChatInput 1963 lines | ChatScreen + ChatInput | Depth gap | Port components incrementally; keep SSE run events | manual chat flow |
| Approvals | Rich cards remember rules | ToolApprovalCard | Partial | Adapted (implemented) | approval flow |
| Settings | Large SettingsView + sections | SettingsScreen + sections | Partial | Adapted (implemented screens) | settings navigation |
| Workspace/git | Full diff review | WorkspaceScreen | Partial | Use Gaggle diff endpoints | diff view |
| Terminal | PTY UI | TerminalScreen | Partial | Use terminal routes | terminal smoke |
| Extensions/MCP | Full management + apps | ExtensionsScreen + apps views | Partial | Use extensions inventory API | extensions screen |
| Recipes/schedules/skills/memories | Full CRUD | Screens exist | Partial | Wire to existing endpoints | CRUD smoke |
| Icons/fonts | Goose assets | Gaggle assets | Licensing | Preserve attribution; use lucide where needed | visual check |
| i18n | formatjs compile step | react-intl + messages | Build step difference | Keep Gaggle i18n | build |

Classification: DIRECT PORT = tokens/styles/primitives; ADAPTED PORT = components with API changes; REIMPLEMENTATION = ACP/Electron-specific; BACKEND DEPENDENCY = needs Gaggle endpoint; NOT APPLICABLE = Electron-only features.