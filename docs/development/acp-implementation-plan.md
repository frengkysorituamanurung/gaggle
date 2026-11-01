# ACP Implementation Plan for Gaggle

## Goal
Make Gaggle's backend compatible with the Agent Client Protocol (ACP) so the
rebranded Electron desktop app can communicate with it.

## Critical Assessment

### What ACP requires
The `agent-client-protocol` crate (v3.6.0) uses JSON-RPC over WebSocket.
A server must implement `HandleDispatchFrom<Client>` which routes incoming
messages. Goose implements this in `crates/goose/src/acp/server.rs` (~4000 lines)
wrapping `StateMachineServices`.

### What Gaggle has
- Axum HTTP/SSE server with REST endpoints
- Agent loop with streaming, approvals, cancellation
- SQLite persistence for chats, runs, messages
- Provider adapter trait
- MCP client integration

### Gap
Gaggle's agent loop uses a different architecture (bounded turn loop with
checkpoints) vs Goose's state machine (operations + effects). The ACP
protocol expects specific message formats and streaming notifications.

## Recommended Approach: Option B (Compatibility Bridge)

Create a compatibility layer that translates ACP requests to Gaggle's REST
API. This is faster than porting Goose's ACP server and leverages Gaggle's
existing infrastructure.

### Architecture
```
Electron Desktop (ACP client)
    ↓ WebSocket /acp
Gaggle ACP Bridge (JSON-RPC handler)
    ↓ REST API calls
Gaggle Axum Server (existing)
    ↓
Gaggle Agent Loop (existing)
```

### Phase 1: Core ACP Methods (Current)
- [x] `/health` and `/status` endpoints
- [x] In-memory session manager
- [x] `initialize` - protocol handshake
- [x] `session/new` - create chat + project
- [x] `session/prompt` - submit message + stream response
- [x] `session/cancel` - cancel run

### Phase 2: Session Management
- [x] `session/list` - list chats
- [x] `session/load` - load chat history
- [x] `session/delete` - delete chat
- [x] `session/close` - close session
- [x] `session/fork` - fork chat

### Phase 3: Configuration
- [x] `session/setConfigOption` - provider, model, mode
- [ ] Provider/model discovery
- [ ] Permission mode switching

### Phase 4: Streaming & Notifications
- [ ] Session update notifications
- [ ] Tool call notifications
- [ ] Usage updates
- [ ] Permission requests

### Phase 5: Advanced Features
- [ ] Recipe support
- [ ] Extension management
- [ ] Scheduler
- [ ] File system access
- [ ] MCP app proxy

## Implementation Strategy

### Step 1: ACP WebSocket Handler
Create a WebSocket handler at `/acp` that:
1. Accepts WebSocket connections
2. Parses JSON-RPC messages
3. Routes to appropriate handlers
4. Sends responses and notifications

### Step 2: Session Bridge
Map ACP sessions to Gaggle chats:
- `session/new` → `POST /api/chats` + `POST /api/projects`
- `session/prompt` → `POST /api/chats/{id}/messages`
- `session/cancel` → `POST /api/runs/{id}/cancel`

### Step 3: Streaming Bridge
Translate Gaggle SSE events to ACP notifications:
- `message.delta` → `session/update` with text content
- `tool.started` → `session/update` with tool call
- `tool.completed` → `session/update` with tool result
- `approval.requested` → `session/request_permission`
- `run.completed` → prompt response with stop_reason

### Step 4: Config Bridge
Translate ACP config options to Gaggle settings:
- `provider` → provider selection
- `model` → model selection
- `mode` → permission mode

## Technical Challenges

1. **Protocol mismatch**: ACP uses JSON-RPC, Gaggle uses REST. Need a translation layer.
2. **Streaming**: ACP uses WebSocket notifications, Gaggle uses SSE. Need to bridge.
3. **Session state**: ACP sessions are stateful, Gaggle chats are persisted. Need mapping.
4. **Provider config**: ACP has complex provider/model config, Gaggle has simpler model selection.
5. **Permissions**: ACP has permission requests, Gaggle has approval flow. Need to bridge.

## Effort Estimate
- Phase 1: 2-3 days
- Phase 2: 2-3 days
- Phase 3: 1-2 days
- Phase 4: 2-3 days
- Phase 5: 3-5 days
Total: 10-16 days

## Dependencies
- `agent-client-protocol` (already added)
- `agent-client-protocol-http` (already added)
- `tokio-tungstenite` (for WebSocket)
- `serde_json` (already available)