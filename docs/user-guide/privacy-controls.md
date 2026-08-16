# Privacy and audit

[Security model](../security/security-model.md) · [Providers](providers.md) · [Logs](../operations/logs-and-diagnostics.md)

**Privacy & Audit** summarizes event categories and recent append-only audit
entries in the active vault. Provider calls, cloud decisions, corpus operations,
memory changes, and permission actions are recorded with structured metadata,
not raw secret values.

Privacy is enforced in the backend: a locked vault blocks private operations;
`local_only` records are filtered before cloud context assembly; a failed local
route never silently becomes an unconfirmed cloud transmission; and Venice keys
never return to React.

PKC enable/disable, health checks, authorization, and retrieval decisions are
audited with identifiers, counts, and hashes — not raw personal evidence.

The first-release tool registry is intentionally restricted to source search,
corpus lookup, draft generation, non-executing planning, and a mock external
tool. It does not send email, browse, purchase, delete files, or execute shell
commands.
