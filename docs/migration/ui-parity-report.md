# UI Parity Report

## Screens compared
- Chat: Gaggle ChatScreen baseline vs Goose BaseChat/ChatInput. Functional SSE streaming, tool cards, approvals present; deep parity (attachments, mentions, slash commands, live voice) not yet.
- Shell/navigation: Gaggle AppLayout/NavigationPanel baseline vs Goose sidebar. Functional.
- Settings: Gaggle SettingsScreen + sections vs Goose SettingsView. Functional subset.
- Providers: ConfigureProvidersScreen vs Goose provider settings. Baseline.
- Secondary screens: skills, recipes, extensions, scheduler, memories, workspace, terminal, diagnostics. Baseline.

## Components migrated (baseline)
ChatInput, ToolCallCard, ToolApprovalCard, ThinkingBlock, MessageBubble, EmptyState, LoadingState, ErrorState, Markdown, AutoTextarea, Button, Card, Collapsible, Dialog, Input, ScrollArea, Skeleton, Switch, Tabs, Tooltip, MentionPopover, MessageQueue, LauncherView, McpAppRenderer, PipWindow.

## Visual differences
- Component density/spacing and exact control shapes differ in places.
- Iconography uses lucide vs goose icons in some spots.
- Reduced-motion/accessibility not fully verified.

## Functional differences
- No file attachments, quote, in-conversation search, live voice integration yet.
- SSE run events drive chat; approval resume requires reload (backend does not auto-resume).
- Desktop startup not verified in this environment.

## Known limitations
- Full 1:1 parity is multi-week; current state is functional baseline wired to real Gaggle backend.
- No Windows support in scope.

## Not verified
- Builds/tests not run (per execution policy, verification is user's responsibility).