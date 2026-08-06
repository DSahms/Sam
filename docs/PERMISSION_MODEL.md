# Permission Model

Sammy's first release is deliberately narrow. The permission system exists
to make that narrowness explicit and enforceable, and to give the owner a
clear, auditable way to widen it later, one action at a time.

## First-release capability set

The first release permits **only** the following:

- **Read-only local source search.** Querying the vault's FTS5/vector indexes
  and reading the owner's own sources to assemble context.
- **Read-only corpus lookup.** Reading knowledge records for retrieval and
  citation.
- **Draft generation.** Producing text drafts the owner can copy, edit, or
  discard.
- **Non-executing planning.** Producing a step-by-step plan with no
  automatic action.
- **Mock tools.** Deterministic in-process tools used for testing and
  demonstration that have no real-world effect.

## Forbidden in the first release

The following are **not** present and must fail-closed if ever invoked:

- Sending email or any other message on the owner's behalf.
- Posting to or interacting with social media.
- Making purchases or moving money.
- Deleting files (the owner's or anyone else's).
- Executing shell commands, scripts, or arbitrary programs.
- Driving a browser to take actions on web sites.
- Any external write or any side effect outside the vault.

There is no escape hatch, no "power user" flag, and no hidden capability
toggle that enables these.

## Tool-action declaration

Every tool action — even the read-only ones permitted above — must declare
its full contract before it may run. The declaration is shown to the owner
at confirmation time and is recorded verbatim in the audit log:

- **tool_id** — stable identifier of the tool.
- **operation** — what the tool will do, in plain language.
- **data_accessed** — which records, sources, or vault data the tool will
  read.
- **data_leaving_device** — what (if anything) will leave the device.
- **destination** — where leaving data goes (provider, endpoint class, or
  "none").
- **risk_level** — `none | low | medium | high`.
- **reversibility** — `reversible | irreversible | unknown`.
- **duration** — one-shot, session, persistent.
- **scope** — which vault, domain, or record set the action affects.
- **confirmation_policy** — one of the modes below.
- **result** — filled in after the action with the outcome and any artifacts.
- **audit_event** — the audit event id under which the action is recorded.

## Permission modes

The confirmation policy for a tool action is one of:

- **always deny** — the tool may never run; attempting it returns an error.
- **ask every time** — prompt the owner before each invocation; never cache
  consent.
- **allow once** — permit this single invocation only; the next invocation
  asks again.
- **allow session** — permit for the remainder of the current unlocked
  session.
- **narrow scope** — permit only for a specific record, domain, or data
  subset named in the grant.
- **read only** — permit read operations only; writes remain gated.
- **draft only** — permit producing drafts; never execute or send.
- **execute with immediate confirmation** — execute, but require the owner
  to confirm the result before it is committed or shown as final.

## Always-require-approval list

A per-vault list of actions that, regardless of any broader grant, always
require an explicit confirmation for every invocation. Examples include any
action marked `risk_level: high` or `reversibility: irreversible`, and any
first invocation of a tool that has not been used before in this vault. The
list is owner-editable and ships with safe defaults.

## No permanent authorization from conversation

A conversational request — even an emphatic one ("you can always send my
email", "from now on just do X without asking") — **does not** create a
permanent authorization. Permission grants are made only through the
explicit settings/confirmation UI, are scoped per the modes above, and are
recorded in the audit log. Sammy must respond to such requests by offering
to open the relevant permission setting, not by silently honoring them.

## Revocation

Any grant may be revoked at any time. Revocation takes effect **immediately**
for all future invocations:

- In-flight invocations are cancelled where reversible.
- Session-scoped grants expire on lock or on explicit revoke.
- Narrow-scope and read-only/draft-only grants are removed from the
  permission table and the next invocation prompts again.

Revocation is itself an audit event.

## Audit

Every tool invocation — permitted, denied, or revoked — produces an audit
event in the active vault containing the full tool-action declaration, the
owner decision (or the denial reason), the result, and the timestamp. The
audit log is per-vault, exportable, and never editable by Sammy itself.
