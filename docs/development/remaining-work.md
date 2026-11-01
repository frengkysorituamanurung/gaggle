# Remaining Work for Full Goose Parity

## Critical Path (requires significant development)

### 1. Full ACP Protocol (HandleDispatchFrom)
- **Effort:** 2-4 weeks
- **Description:** Port Goose's `HandleDispatchFrom<Client>` trait implementation (4000+ lines) from `goose/crates/goose/src/acp/server.rs`
- **Approach:** Either port wholesale or create compatibility bridge
- **Dependencies:** `agent-client-protocol`, `agent-client-protocol-http`
- **Status:** Minimal ACP WebSocket handler exists with all core methods

### 2. Local Inference (llama.cpp)
- **Effort:** 1-2 weeks
- **Description:** Add `gaggle-local-inference` crate wrapping llama.cpp
- **Features:** Optional GPU backends (CUDA, Vulkan, Metal, MLX)
- **Status:** Not started. Ollama is the only local inference option.

### 3. OIDC Proxy
- **Effort:** 1 week
- **Description:** Add OIDC proxy for enterprise auth
- **Status:** Not started

### 4. Download Manager
- **Effort:** 1 week
- **Description:** Centralized model/asset download with progress
- **Status:** Not started

### 5. Slash Commands
- **Effort:** 3-5 days
- **Description:** Slash command handling in agent loop
- **Status:** Not started

### 6. Full Hooks
- **Effort:** 3-5 days
- **Description:** Pre/post tool use hooks, stop hooks, etc.
- **Status:** Partial (pre-tool-use hook exists)

### 7. MOIM (Model-Owned Instruction Memory)
- **Effort:** 1 week
- **Description:** Model-owned instruction memory system
- **Status:** Not started

### 8. Final Output Tool
- **Effort:** 2-3 days
- **Description:** Dedicated final output tool for structured results
- **Status:** Not started

### 9. Snapshots
- **Effort:** 3-5 days
- **Description:** Session snapshots for undo/redo
- **Status:** Not started

### 10. GenAI Telemetry
- **Effort:** 3-5 days
- **Description:** OpenTelemetry integration
- **Status:** Basic tracing exists

### 11. Computer Controller
- **Effort:** 1-2 weeks
- **Description:** macOS automation (peekaboo)
- **Status:** Not started

### 12. Adversary Mode
- **Effort:** 3-5 days
- **Description:** Adversarial testing mode
- **Status:** Not started

### 13. Live Voice Backend
- **Effort:** 1 week
- **Description:** Voice interaction backend
- **Status:** UI exists, backend not implemented

### 14. ChatGPT OAuth
- **Effort:** 3-5 days
- **Description:** ChatGPT OAuth PKCE flow
- **Status:** Adapter exists, OAuth flow partially implemented

### 15. Native Menu Localization (all locales)
- **Effort:** 2-3 days
- **Description:** Menu translations for all 16 locales
- **Status:** Framework created (i18n-menu.ts), Chinese translations exist

### 16. McpApps Renderer
- **Effort:** 2-3 days
- **Description:** Full MCP app rendering with message passing
- **Status:** Basic renderer with iframe sandbox created