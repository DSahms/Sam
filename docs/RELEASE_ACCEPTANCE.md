# Release Acceptance — Definition of Finished

This document defines what "finished" means for Sammy's first release and
lists, as checklists, every acceptance criterion and every required
end-to-end test. A roadmap entry, a mockup, or a passing unit test does
**not** satisfy "finished". A feature is finished only when it is
implemented, integrated, covered by the E2E workflows below, and absent
from the forbidden-behavior list.

## What "finished" is not

- A roadmap line is not finished.
- A Figma or ASCII mockup is not finished.
- A passing unit test for an isolated function is not finished.
- A behind-a-feature-flag implementation that the E2E suite does not
  exercise is not finished.

A feature counts as finished only when all of the following hold:

1. The code is implemented and merged with no `make check` failures.
2. The relevant E2E workflow in this document passes on a clean Windows 11
   machine.
3. The forbidden-behavior tests in this document pass.
4. Documentation in this `docs/` set accurately describes the shipped
   behavior.

## Acceptance criteria checklist

Each must be true in the shipped build.

### Vaults and keys
- [ ] Owner can create a new vault with a master passphrase and a template.
- [ ] Vault creation generates and displays the recovery key exactly once,
      and requires owner confirmation of the recovery key before the vault
      is usable.
- [ ] Unlock with the correct passphrase opens the vault; a wrong passphrase
      is rejected with a generic error and no leak.
- [ ] Recovery with the recovery key unlocks the vault and allows setting a
      new passphrase.
- [ ] Locking a vault drops the VDEK from memory and closes the database.
- [ ] No plaintext passphrase, VDEK, or recovery key is ever written to
      disk or to logs.

### Isolation
- [ ] Two vaults on the same machine cannot retrieve from each other.
- [ ] No shared SQLCipher database, FTS5 table, or vector index exists
      across vaults.
- [ ] The only cross-vault transfer path is audited export + separately
      confirmed audited import.

### Corpus
- [ ] Sources can be imported, encrypted at rest, and searched via FTS5.
- [ ] Knowledge records can be created, edited, superseded, and tombstoned.
- [ ] Indexes can be rebuilt from Layer 1 without losing data.
- [ ] Export produces an encrypted package; importing the same package
      twice produces no duplicates.

### Retrieval
- [ ] A retrieval pass executes all 11 steps in order.
- [ ] Retrieval refuses to run without an unlocked vault.
- [ ] Local-only records are never sent to a cloud provider.

### Memory
- [ ] Conversation content does not become durable memory automatically.
- [ ] Memory candidates can be approved, edited+approved, rejected,
      deferred, marked temporary, converted to corrections, superseded, or
      deleted.
- [ ] Rejected candidates never appear in retrieval.

### Permissions
- [ ] First-release capability set is limited to read-only source search,
      read-only corpus lookup, draft generation, non-executing planning,
      and mock tools.
- [ ] No external execution capability (email, social, purchases, file
      deletion, shell, browser) is present.
- [ ] Every tool action declares its full contract and is recorded in the
      audit log.
- [ ] No conversational request grants permanent authorization.
- [ ] Permission revocation takes effect immediately.

### Providers
- [ ] Provider swap does not require changes outside the provider layer.
- [ ] Three adapters ship: deterministic mock, KoboldCpp-compatible local,
      Venice-compatible cloud.
- [ ] Every cloud transmission is an audit event.
- [ ] Errors never log the private prompt.

### Backup and recovery
- [ ] Backup writes an encrypted package to an owner-selected location.
- [ ] Restore works on a fresh machine with only the master passphrase or
      recovery key, without the original OS account, keychain, or any
      online service.
- [ ] Wrong credential, corrupted package, unsupported version, and
      interrupted restore each have defined, user-visible handling.

## Required end-to-end workflows (spec §36)

Each workflow must pass as a single scripted, reproducible scenario on a
clean machine.

- [ ] **W1 New vault.** Install → create vault (passphrase + template) →
      capture recovery key → confirm recovery key → land on empty vault.
- [ ] **W2 Unlock and lock.** Lock → unlock with passphrase → confirm
      retrieval works → lock → confirm retrieval refuses.
- [ ] **W3 Recovery.** With vault locked, choose "forgot passphrase" →
      enter recovery key → set new passphrase → unlock with new passphrase
      → confirm prior data intact.
- [ ] **W4 Import source and search.** Import a source file → confirm it is
      encrypted at rest → search its contents via FTS5 → confirm citation
      resolves to the source.
- [ ] **W5 Knowledge record lifecycle.** Create record → edit → supersede
      → confirm old record no longer in retrieval, new one is.
- [ ] **W6 Memory candidate.** Produce a candidate from a conversation →
      reject one, approve one, defer one → confirm only the approved one
      appears in retrieval.
- [ ] **W7 Retrieval isolation.** With two vaults, place a record only in
      vault A → confirm a retrieval in vault B cannot surface it.
- [ ] **W8 Provider routing.** Configure local + cloud adapters → set mode
      `ask_before_crossing` → issue a request that would cross → confirm
      Sammy asks before sending and that a cloud send produces an audit
      event.
- [ ] **W9 Permission gate.** Invoke a tool action → confirm the
      tool-action declaration is shown → approve once → confirm a second
      invocation asks again.
- [ ] **W10 Backup and restore to a new machine.** Back up to a folder →
      on a fresh install, restore from the package using only the
      passphrase (or recovery key) → confirm vault contents match.
- [ ] **W11 Cross-vault export + import.** Export records from vault A →
      confirm source audit log records the export → import into vault B
      with separate confirmation → confirm destination audit log records
      the import → confirm re-importing the same package produces no
      duplicates.
- [ ] **W12 Index rebuild.** Delete the vector + FTS5 indexes → trigger
      rebuild → confirm all records still retrievable.

## Forbidden-behavior tests (spec §37)

Each must be a passing test asserting the behavior never occurs.

- [ ] **F1 No auto-memory.** A conversation containing an obvious fact does
      not, after lock, appear as a knowledge record.
- [ ] **F2 No cross-vault retrieval.** A query in vault B never returns a
      record unique to vault A.
- [ ] **F3 No shared embedding index.** Inspecting per-vault storage
      confirms two distinct physical index files for two vaults.
- [ ] **F4 No silent cloud fallback.** A failed local provider is not
      retried against a cloud provider without owner action.
- [ ] **F5 Local-only never sent to cloud.** A record tagged `local_only`
      is stripped before any cloud provider call; the test asserts the
      sent payload does not contain it.
- [ ] **F6 No prompt in error logs.** A forced provider error produces a
      log entry that does not contain the original prompt text.
- [ ] **F7 No permanent grant from chat.** Telling Sammy "always do X
      without asking" in chat does not change the permission table.
- [ ] **F8 No external execution.** Attempting any external-effect tool
      (email, shell, file deletion, browser action, purchase) fails closed.
- [ ] **F9 No plaintext credentials at rest.** Provider API keys are not
      present in plaintext in any on-disk file.
- [ ] **F10 No third unwrap path.** With both passphrase and recovery key
      unknown, no code path — including a vendor support path — can unwrap
      the VDEK.
- [ ] **F11 No overwrite without confirmation.** Restoring a package does
      not replace the live vault without an explicit owner confirmation.
- [ ] **F12 No duplicate on re-import.** Importing the same package a
      second time creates zero new records and zero modified records.
