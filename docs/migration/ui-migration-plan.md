# UI Migration Plan (Goose → Gaggle)

## Phases
1. Reconnaissance (done): inventories + compatibility matrix.
2. Compatibility engineering: REST/SSE adapter, Tauri shell.
3. UI foundation: tokens, primitives, styles (already aligned).
4. Application shell: sidebar, navigation, routing (implemented).
5. Chat UI: ChatScreen, ChatInput, tool cards, approvals, streaming (implemented baseline).
6. Secondary screens: settings, providers, skills, recipes, extensions, scheduler, memories, workspace, terminal, diagnostics (implemented baseline).
7. Backend adapter: api.ts wiring (implemented).
8. Desktop integration: Tauri startup, asset loading (scripts present).
9. Visual parity & regression: ongoing.
10. Final stabilization: docs, checks.

## Checklist
- [x] Docs inventories created
- [x] Compatibility matrix created
- [x] Chat streaming + tool calls + approvals (baseline)
- [x] Deep chat: BaseChat, enhanced ChatInput (attachments, slash commands, quote, search-in-conversation, live voice), ChatInputCard, ChatSessionsContainer
- [x] Full ChatInput parity pass: mentions, message queue, image paste, file drop, command history, cost/context indicators, working dir, appendQuote, queued send-now/clear
- [x] Mention items populated from skills + project files; command history persisted to localStorage; queue cleared on run finalize; attachments referenced in prompt
- [x] ChatInput keyboard navigation (slash/mention popovers), focus-visible, reduced-motion, toolIconMapping dynamic
- [x] Backend attachment support: SubmitChatMessageRequest.attachments + persisted message parts; frontend api.startRun sends attachments
- [x] Hooks implemented: useChatSession, useAutoSubmit, useAudioRecorder, useFocusOnTyping, useNavigationSessions, useProviderDeviceCode
- [x] Toolcall icons expanded (17 icons + index)
- [x] Accessibility baseline: focus-visible rings in Button; reduced-motion media query added to styles.css
- [x] LiveVoiceButton wired to useLiveVoice controller (Web Speech API)
- [x] CI/CD updated: frontend job (tsc/build/test) in ci.yml; ui build step in reusable-build.yml; release.yml frontend verify; setup-node action fixed
- [x] cargo fmt issues in acp.rs manually aligned to rustfmt (remaining minor diffs may need `cargo fmt --all`)
- [x] Visual/brand icons expanded further; LoadingGoose/FlyingBird/GooseLogo/AnimatedIcons improved
- [ ] i18n compile step (formatjs) — not adopted; Gaggle uses react-intl runtime
- [ ] Desktop startup, asset loading, packaging — not verified in this environment
- [x] Bottom menu: CostTracker, ContextWindowIndicator, DirSwitcher, ExtensionMenu, BottomMenuExtensionSelection, BottomMenuAlertPopover
- [x] Deep recipes: RecipesView, RecipeHeader, RecipeActivities, RecipeActivityEditor, CreateEditRecipeModal, ImportRecipeForm, SubRecipeModal, CreateSubRecipeInline, RecipeParamsModalContainer, RecipeConsentModalContainer
- [x] Deep extensions: ExtensionsView, ExtensionInstallModal, ExtensionModal, ExtensionConfigFields, HeadersSection, EnvVarsSection
- [x] Deep settings: SettingsView, ProviderSettingsPage, ProviderConfigurationModal, ProviderGrid, ProviderCard, AcpReadinessPanel, HuggingFaceSignInPrompt, PermissionModal, PermissionRulesModal, GoosehintsModal, ResetProviderSection, UpdateSection, TelemetrySettings, ConfigSettings, ConversationLimitsDropdown, ConfigureApproveMode, ModeSelectionItem, ModelsBottomBar, SwitchModelModal, ModelSettingsButtons, DictationSettings, LocalInferenceSettings, HuggingFaceModelSearch, ModelSettingsPanel
- [x] Sessions: SessionListView, SessionsView
- [x] Schedule: SchedulesView, ScheduleModal, CronPicker, ScheduleDetailView
- [x] Skills: SkillsView
- [x] Context management: SystemNotificationInline, CreditsExhaustedNotification (ContextManagementPanel existing)
- [x] Parameter: ParameterInput (ParameterInputModal existing)
- [x] Alerts: AlertBox, GroupedExtensionLoadingToast
- [x] Visual/brand: LoadingGoose, FlyingBird, GooseLogo, GooseMessage, AnimatedIcons, ItemIcon, bird icons, toolcall icons
- [x] Util: toolIconMapping, InlineEditText
- [x] Batch: MessageList, UserMessage, MessageUsageStats, MessageCopyLink, ToolCallWithResponse, ToolCallArguments, ToolCallStatusIndicator, ThinkingContent, GitBranchIndicator, ErrorBoundary
- [x] Batch: Hooks (useFileDrop, useEscapeKey, useIsMobile/use-mobile, useNavigation, useAlerts, useAutoSubmit, useChatSession, useFocusOnTyping, useAudioRecorder, useNavigationSessions, useProviderDeviceCode)
- [x] Batch: GooseSidebar/EnvironmentBadge
- [x] Batch: Settings subcomponents (providers modal/subcomponents, providers subcomponents, extensions modal/subcomponents, dictation, response_styles, config)
- [x] Batch: Recipes shared subcomponents
- [x] Batch: Extensions subcomponents & modal fields
- [x] Batch: Alerts (useAlerts, AlertBox)
- [x] Batch: Infra/UI (SuspenseLoader, toasts)
- [x] Sidebar navigation (baseline)
- [x] Provider/model management (baseline)
- [x] Settings sections (baseline)
- [x] Workspace/diff (baseline)
- [x] Terminal (baseline)
- [x] Extensions/apps (baseline)
- [x] Recipes/schedules/skills/memories (baseline)
- [x] Onboarding/diagnostics (baseline)
- [ ] Full 1:1 visual parity (ongoing)
- [ ] Desktop startup verification (requires build env)