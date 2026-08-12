# Architecture overview

[Documentation home](../README.md) · [Repository layout](../development/repository-layout.md) · [Security boundaries](security-boundaries.md)

Sammy is a Tauri 2 desktop application. React renders the interface and invokes a
typed command surface. Rust owns private state, cryptography, storage, provider
calls, retrieval, and audit decisions.

```mermaid
flowchart TB
    subgraph UI["React / TypeScript"]
      V["Eight navigation views"]
      T["Typed invoke wrappers"]
    end
    subgraph CORE["Rust / Tauri"]
      A["AppState: active vault"]
      D["Domain services"]
      P["Provider routing"]
    end
    subgraph DATA["Per-vault boundary"]
      M["vault.json: wrapped key metadata"]
      DB["vault.db: SQLCipher"]
      IX["FTS5 + vector index"]
    end
    V --> T --> A --> D
    D --> DB
    D --> IX
    A --> M
    P --> K["KoboldCpp"]
    P -->|"consented HTTPS"| VN["Venice"]
```

`AppState` holds at most one unlocked `Vault`; dropping it closes the database
connection and zeroizing secret-key object. Commands return sanitized DTOs, not
connections or key material. Forward-only transactional migrations evolve each
vault independently.
