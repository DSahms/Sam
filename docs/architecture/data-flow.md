# Data flow

[Architecture overview](overview.md) · [Provider architecture](provider-architecture.md)

```mermaid
sequenceDiagram
    participant O as Owner
    participant UI as React UI
    participant R as Rust command
    participant C as Corpus/retrieval
    participant P as Provider
    participant A as Audit
    O->>UI: Send message
    UI->>R: chat_send(conversation, text, routing, model, confirmed)
    R->>C: Retrieve current permitted context
    C-->>R: Ranked records + stable IDs
    R->>R: Assemble identity and prompt
    R->>P: Route and call local or consented cloud provider
    P-->>R: Response + crossed_to_cloud
    R->>A: Record provider decision/result
    R-->>UI: Sanitized response and inspection summary
```

The UI supplies intent and confirmation; it does not decide what private records
may cross the provider boundary. The backend filters, routes, and audits.
