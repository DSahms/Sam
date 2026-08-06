# Sammy — Test Plan

Testing strategy across the stack. Required test categories come from
directive §4, §36, §37. Acceptance is in `docs/RELEASE_ACCEPTANCE.md`.

## Test layers

| Layer              | Framework              | Where                      |
| ------------------ | ---------------------- | -------------------------- |
| Rust unit          | `cargo test`           | `#[cfg(test)] mod tests`   |
| Rust integration   | `cargo test`           | `tests/` (Phase 1+)         |
| TS unit / component| Vitest + Testing Lib   | `src/**/*.test.tsx`        |
| DB migration       | Rust integration       | `tests/migrations.rs`      |
| Corpus contract    | Rust + fixtures        | `tests/corpus_contract.rs` |
| Provider adapter   | Rust + mock            | `tests/providers.rs`       |
| Retrieval          | Rust integration       | `tests/retrieval.rs`       |
| Security regression| Rust integration       | `tests/security/*.rs`      |
| Backup/restore     | Rust integration       | `tests/backup.rs`          |
| E2E (where practical) | Tauri + manual runbook | `docs/RELEASE_ACCEPTANCE.md` |

## Commands

```bash
make test           # cargo test --workspace && npm run test
make check          # fmt + clippy + eslint + prettier + tsc + test (CI gate)
```

## Required end-to-end workflows (directive §36)

Each is an acceptance workflow, enumerated in `docs/RELEASE_ACCEPTANCE.md`:
new vault; recovery; multiple vaults; source-grounded answer; OCR;
contradiction; memory; provider privacy; corpus package; permissions.

## Forbidden-behavior tests (directive §37)

Automated tests asserting Sammy does **not** permit:
private/chat/source access before unlock; cross-vault retrieval or indexing;
silent cloud fallback or transmission; duplicate corpus imports; retrieval of
tombstoned records or deleted source chunks; automatic approval of AI memory;
fake citation IDs; tool execution without permission; permanent authorization
from ordinary conversation; plaintext secrets in logs; plaintext vault keys in
frontend state; plaintext sources left in temp folders; restore from corrupted
data without warning; migration without rollback protection; external repo
inspection; arbitrary filesystem scanning; automatic telemetry.

Failing security tests are **never** disabled to obtain a green build.
