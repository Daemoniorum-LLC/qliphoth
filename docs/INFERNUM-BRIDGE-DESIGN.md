# Qliphoth-Infernum Bridge Design

**Version:** 1.0
**Date:** 2024-12-26
**Status:** In Development

## Overview

This document describes the full-featured bridge between Qliphoth (Sigil web platform) and Infernum (local LLM inference engine). The bridge provides capabilities matching the most advanced chat platforms in the Daemoniorum ecosystem.

## Feature Parity Matrix

| Feature | Persona Framework | Daemoniorum App | Bael | **Qliphoth** |
|---------|-------------------|-----------------|------|--------------|
| **Protocol** |
| HTTP REST | ✅ | ✅ | ✅ | ❌ (WebSocket only) |
| SSE Streaming | ✅ | ✅ | ✅ | ❌ |
| WebSocket | ❌ | ❌ | ❌ | ✅ |
| **Messages** |
| Multi-turn | ✅ | ✅ | ✅ | ✅ |
| History Persistence | ✅ | ✅ | ✅ | ✅ (planned) |
| Edit/Regenerate | ✅ | ✅ | ✅ | ✅ (planned) |
| Cancel/Abort | ❌ | ❌ | ❌ | ✅ |
| **Tool Calling** |
| Tool Definitions | ✅ | ❌ | ❌ | ✅ (planned) |
| Tool Result Display | ✅ | ❌ | ❌ | ✅ (planned) |
| Approval Workflow | ❌ | ❌ | ❌ | ✅ (planned) |
| **Observability** |
| Token Tracking | ✅ | ✅ | ❌ | ✅ (planned) |
| Latency Metrics | ❌ | ❌ | ❌ | ✅ (planned) |
| Request IDs | ❌ | ❌ | ❌ | ✅ |
| **Advanced** |
| Thinking Blocks | ❌ | ❌ | ❌ | ✅ (planned) |
| Persona Routing | ✅ | ✅ | ✅ | ✅ |
| Context Awareness | ❌ | ❌ | ❌ | ✅ |
| Visual Effects | ❌ | ❌ | ❌ | ✅ |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Qliphoth Chat                             │
├─────────────────────────────────────────────────────────────────┤
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────────────┐│
│  │   ChatWidget  │  │ MessageList   │  │   ToolCallRenderer    ││
│  │   (UI Layer)  │  │ (Display)     │  │   (Tool Results)      ││
│  └───────┬───────┘  └───────┬───────┘  └───────────┬───────────┘│
│          │                  │                      │             │
│  ┌───────┴──────────────────┴──────────────────────┴───────────┐│
│  │                    ChatProvider (State)                      ││
│  │  - messages: Signal[Vec[ChatMessage]]                        ││
│  │  - pending_requests: Map[RequestId, PendingRequest]          ││
│  │  - tool_approvals: Signal[Vec[ToolApproval]]                 ││
│  │  - metrics: ChatMetrics                                      ││
│  └───────┬──────────────────────────────────────────────────────┘│
│          │                                                       │
│  ┌───────┴──────────────────────────────────────────────────────┐│
│  │                 InfernumProtocol (Bridge)                    ││
│  │  - parse_server_message() -> ServerMessage                   ││
│  │  - format_client_message() -> String                         ││
│  │  - handle_delta() / handle_done() / handle_error()           ││
│  └───────┬──────────────────────────────────────────────────────┘│
│          │                                                       │
│  ┌───────┴──────────────────────────────────────────────────────┐│
│  │                 InfernumClient (WebSocket)                   ││
│  │  - connect() / disconnect() / reconnect()                    ││
│  │  - send_chat() / send_cancel() / send_ping()                 ││
│  │  - connection_state: Signal[ConnectionState]                 ││
│  └───────┬──────────────────────────────────────────────────────┘│
│          │                                                       │
└──────────┼───────────────────────────────────────────────────────┘
           │ WebSocket
           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Infernum Server                              │
│  ws://localhost:8081/ws/chat                                     │
├─────────────────────────────────────────────────────────────────┤
│  Endpoints:                                                      │
│  - /ws/chat          → Chat WebSocket                           │
│  - /v1/chat/completions → HTTP (fallback)                       │
│  - /health           → Health check                              │
│  - /metrics          → Prometheus metrics                        │
└─────────────────────────────────────────────────────────────────┘
```

## Protocol Specification

### Client → Server Messages

```sigil
/// All client messages are tagged JSON
pub type ClientMessage = enum {
    /// Chat completion request
    Chat {
        payload: ChatCompletionRequest!,
        request_id: String!,
    },

    /// Cancel in-flight request
    Cancel {
        request_id: String!,
    },

    /// Keepalive ping
    Ping {
        timestamp: u64?,
    },

    /// Tool approval response
    ToolApproval {
        request_id: String!,
        tool_call_id: String!,
        approved: bool!,
    },
}

pub type ChatCompletionRequest = struct {
    pub model: String!,
    pub messages: Vec[ChatMessage]!,
    pub temperature: f32?,
    pub top_p: f32?,
    pub max_tokens: u32?,
    pub stream: bool!,
    pub tools: Vec[ToolDefinition]?,
    pub tool_choice: ToolChoice?,
}
```

### Server → Client Messages

```sigil
/// All server messages are tagged JSON
pub type ServerMessage = enum {
    /// Connection established
    Connected {
        connection_id: String!,
        timestamp: u64!,
    },

    /// Streaming content delta
    Delta {
        request_id: String!,
        index: u32!,
        content: String?,
        role: String?,
    },

    /// Thinking/reasoning block
    Thinking {
        request_id: String!,
        content: String!,
    },

    /// Tool call request
    ToolCall {
        request_id: String!,
        tool_call_id: String!,
        name: String!,
        arguments: String!,  // JSON
        requires_approval: bool!,
    },

    /// Tool call result
    ToolResult {
        request_id: String!,
        tool_call_id: String!,
        result: String!,
        is_error: bool!,
    },

    /// Request completed
    Done {
        request_id: String!,
        finish_reason: String!,
        usage: TokenUsage?,
    },

    /// Error occurred
    Error {
        request_id: String?,
        code: String!,
        message: String!,
    },

    /// Pong response
    Pong {
        client_timestamp: u64?,
        server_timestamp: u64!,
    },

    /// Request cancelled
    Cancelled {
        request_id: String!,
    },
}

pub type TokenUsage = struct {
    pub prompt_tokens: u32!,
    pub completion_tokens: u32!,
    pub total_tokens: u32!,
}
```

## Message Types

### ChatMessage (Display)

```sigil
pub type ChatMessage = struct {
    /// Unique message ID
    pub id: String!,

    /// Message role
    pub role: MessageRole!,

    /// Text content
    pub content: String!,

    /// Timestamp
    pub timestamp: u64!,

    /// Associated request ID (for correlation)
    pub request_id: String?,

    /// Message status
    pub status: MessageStatus!,

    /// Thinking/reasoning blocks
    pub thinking: Vec[ThinkingBlock]?,

    /// Tool calls in this message
    pub tool_calls: Vec[ToolCall]?,

    /// Tool results (for tool role messages)
    pub tool_results: Vec[ToolResult]?,

    /// Token usage
    pub usage: TokenUsage?,

    /// Model used
    pub model: String?,

    /// Latency (ms from send to first token)
    pub latency_ms: u32?,

    /// Parent message ID (for regenerations)
    pub parent_id: String?,

    /// Is this an edited version
    pub is_edited: bool!,
}

pub type MessageRole = enum {
    User,
    Assistant,
    System,
    Tool,
}

pub type MessageStatus = enum {
    Pending,      // Waiting to send
    Sending,      // Being sent to server
    Streaming,    // Receiving response
    Complete,     // Finished successfully
    Error(String), // Failed with error
    Cancelled,    // User cancelled
}

pub type ThinkingBlock = struct {
    pub content: String!,
    pub timestamp: u64!,
}
```

## Tool Calling

### Tool Definition

```sigil
pub type ToolDefinition = struct {
    pub name: String!,
    pub description: String!,
    pub parameters: JsonSchema!,
    pub risk_level: RiskLevel!,
}

pub type RiskLevel = enum {
    Safe,           // No approval needed
    Low,            // Approval optional
    Medium,         // Approval recommended
    High,           // Approval required
    Critical,       // Always requires approval + confirmation
}
```

### Tool Approval Flow

```
1. Server sends ToolCall with requires_approval = true
2. UI shows approval dialog with tool name, args, risk level
3. User approves or rejects
4. Client sends ToolApproval message
5. Server executes tool (if approved) and sends ToolResult
6. Server continues generation with tool result
```

## Metrics

### ChatMetrics

```sigil
pub type ChatMetrics = struct {
    /// Total requests made
    pub requests_total: u64!,

    /// Successful completions
    pub completions_total: u64!,

    /// Errors encountered
    pub errors_total: u64!,

    /// Requests cancelled
    pub cancellations_total: u64!,

    /// Total tokens used
    pub tokens_total: TokenUsage!,

    /// Average latency (first token)
    pub avg_latency_ms: f64!,

    /// Tool calls made
    pub tool_calls_total: u64!,

    /// Tool approvals granted
    pub tool_approvals_total: u64!,

    /// Tool rejections
    pub tool_rejections_total: u64!,

    /// Connection state changes
    pub connection_events: Vec[ConnectionEvent]!,
}

pub type ConnectionEvent = struct {
    pub timestamp: u64!,
    pub state: ConnectionState!,
    pub reason: String?,
}
```

## File Structure

```
qliphoth/crates/qliphoth-chat/src/
├── lib.sigil              # Module exports, ChatProvider
├── protocol.sigil         # Infernum protocol types (NEW)
├── client.sigil           # WebSocket client (upgraded)
├── messages.sigil         # ChatMessage types (NEW)
├── tools.sigil            # Tool calling support (NEW)
├── metrics.sigil          # Observability (NEW)
├── streaming.sigil        # Stream handling (upgraded)
├── widget.sigil           # Chat widget UI
├── adaptive.sigil         # Adaptive persona system
├── context.sigil          # Page context awareness
├── markdown.sigil         # Markdown rendering
├── persona.sigil          # Persona management
└── history.sigil          # Conversation persistence (NEW)
```

## Implementation Phases

### Phase 1: Protocol (Current)
- [ ] Create `protocol.sigil` with full message types
- [ ] Upgrade `client.sigil` to new protocol
- [ ] Add request ID correlation
- [ ] Add cancel support

### Phase 2: Messages
- [ ] Create `messages.sigil` with rich message types
- [ ] Add message status tracking
- [ ] Implement edit/regenerate
- [ ] Add parent ID tracking

### Phase 3: Tool Calling
- [ ] Create `tools.sigil` with tool types
- [ ] Add tool call rendering
- [ ] Implement approval workflow
- [ ] Integrate with Beleth agent tools

### Phase 4: Metrics
- [ ] Create `metrics.sigil`
- [ ] Add token tracking
- [ ] Add latency measurement
- [ ] Export to Prometheus format

### Phase 5: Persistence
- [ ] Create `history.sigil`
- [ ] Add IndexedDB storage
- [ ] Implement conversation sync
- [ ] Add export/import

## Visual Effects Integration

The chat integrates with `qliphoth-fx` for visual feedback:

| Event | Effect |
|-------|--------|
| Message sending | Particles burst from input |
| Token received | Subtle glow pulse |
| Tool call | Sigil animation |
| Tool approved | Green aura flash |
| Tool rejected | Red glitch |
| Error | Screen shake + glitch |
| Thinking block | Pulsing orb |

## Security Considerations

1. **Tool Approval**: High-risk tools always require explicit approval
2. **Input Validation**: All inputs validated before sending
3. **Rate Limiting**: Client-side rate limiting (configurable)
4. **Connection Security**: WSS in production
5. **Request Correlation**: All requests tracked by ID

## Configuration

```sigil
pub type ChatConfig = struct {
    /// Infernum WebSocket URL
    pub infernum_url: String!,           // Default: "ws://localhost:8081/ws/chat"

    /// Default model
    pub default_model: String!,          // Default: "qwen2.5-coder-7b"

    /// Enable streaming
    pub streaming: bool!,                // Default: true

    /// Max context tokens
    pub max_context: usize!,             // Default: 8192

    /// System prompt
    pub system_prompt: String!,

    /// Auto-reconnect on disconnect
    pub auto_reconnect: bool!,           // Default: true

    /// Max reconnect attempts
    pub max_reconnect_attempts: u32!,    // Default: 5

    /// Reconnect base delay (ms)
    pub reconnect_delay_ms: u32!,        // Default: 1000

    /// Enable tool calling
    pub enable_tools: bool!,             // Default: true

    /// Tool approval mode
    pub tool_approval_mode: ToolApprovalMode!,

    /// Enable metrics collection
    pub enable_metrics: bool!,           // Default: true

    /// Ping interval (ms)
    pub ping_interval_ms: u32!,          // Default: 30000
}

pub type ToolApprovalMode = enum {
    /// Never require approval
    None,
    /// Only high/critical risk
    HighRiskOnly,
    /// All tool calls
    All,
    /// Per-tool configuration
    PerTool,
}
```

## Next Steps

1. Implement `protocol.sigil` with full Infernum protocol
2. Upgrade `client.sigil` with new message handling
3. Add tool calling UI components
4. Create metrics dashboard
5. Implement conversation persistence
