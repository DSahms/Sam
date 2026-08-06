# Provider Architecture — Replaceable Brains

A *provider* is the component that turns a prepared prompt into a model
response. Sammy is built so the provider can be swapped without touching
anything above the provider interface: the corpus, the retrieval pipeline,
the permission system, the audit log, and the UI all stay the same. The
working motto is **"Keep the soul, change the brain."** The soul is the
owner's vault; the brain is whichever model the owner has routed a request
to.

## The stable provider interface

Every adapter implements one stable trait. The interface is deliberately
small and synchronous in shape (request in, stream out) so that local,
cloud, and mock adapters are interchangeable.

The interface must provide:

- **Chat completion** — accept a structured request (messages, tools, params)
  and produce a response.
- **Streaming** — yield tokens / chunks as they arrive so the UI can render
  incrementally and the user can cancel mid-stream.
- **Model listing** — return the models the provider exposes, with their
  context windows, max output tokens, and declared capabilities.
- **Connection test** — a cheap, side-effect-free probe used by settings to
  confirm a provider is reachable and credentials are valid.
- **Timeouts** — every call has a configurable timeout; no call may hang
  forever.
- **Cancellation** — an in-flight stream can be cancelled by the caller; the
  provider must stop promptly and free resources.
- **Token / context estimates** — a pre-flight estimate of how many tokens
  the request will consume and how much output budget remains, so the
  retrieval pipeline can budget context.
- **Structured errors** — a typed error enum (`auth`, `rate_limit`,
  `network`, `server`, `cancelled`, `content_policy`, `context_too_long`,
  `unknown`) so the caller can react instead of parsing strings.
- **Privacy classification** — each provider declares whether it is `local`
  or `cloud`, and the routing layer treats that declaration as authoritative.
- **Metadata** — provider id, adapter version, model id, endpoint, latency
  for the last call. This metadata is attached to the audit event for every
  request.

## Phase 2 adapters

Three adapters ship with the first connected release:

- **Deterministic mock.** No network. Returns canned or seeded responses.
  Used for tests, for offline demo, and as the default so the app is fully
  usable before any provider is configured.
- **KoboldCpp-compatible local.** Connects to a locally running
  KoboldCpp-style endpoint over loopback. Treated as `local`; requests to it
  never leave the device.
- **Venice-compatible cloud.** Connects to a Venice-compatible cloud
  endpoint over HTTPS. Treated as `cloud`; every request is an audit event
  and is subject to routing and permission gates.

The adapter list is open: a future Ollama, llama.cpp, or other adapter can
be added by implementing the trait, without changes to callers.

## Routing modes

The owner chooses a routing mode per vault. The mode decides, for each
request, which class of provider is permitted:

- **`local_only`** — only `local` providers may be used. If no local
  provider is configured, the request fails rather than fall back to cloud.
- **`prefer_local`** — use a local provider if available; fall back to cloud
  only with owner-configured consent, never silently.
- **`prefer_cloud`** — use a cloud provider if available; fall back to local
  only with owner-configured consent.
- **`cloud_only`** — only `cloud` providers may be used.
- **`ask_before_crossing`** *(default)* — if the natural choice (e.g., the
  best model for the task) would cross the local/cloud boundary that the
  owner has not already authorized for this kind of request, Sammy pauses
  and asks the owner before sending.

## Routing rules (non-negotiable)

These hold regardless of the chosen mode:

1. **Local-only requests never go to cloud.** A request the owner marked
   `local_only` (for example, one that includes a record tagged
   `sensitivity: intimate`) is rejected before it can reach a cloud
   provider, even if that means no response is produced.
2. **No silent fallback.** A failed provider is never retried against a
   different privacy class without owner action. Fallback inside the same
   class is allowed.
3. **Cloud requests carry minimal context.** Before sending to a cloud
   provider, the context set is reduced to what is strictly necessary for
   the turn, and records marked `local_only` are stripped. The reduction is
   logged.
4. **Every cloud transmission is an audit event.** The event records
   provider, model, count of records sent, sensitivity tags of sent
   records, timestamp, and outcome. No event records the full private
   prompt text.
5. **Credentials are encrypted.** Provider API keys live in the vault's
   SQLCipher database, never in plaintext config files, and never in the
   OS credential store unencrypted.
6. **Errors must not log the private prompt.** When a provider returns an
   error, the logged diagnostic contains only the typed error, the request
   metadata, and the provider's own error message — never the original
   prompt or retrieved context.

## Interaction with permissions

Provider choice and permission grants are separate but cooperative. A tool
that is permitted only in `draft_only` mode can still be served by any
provider, but if the request would cross to cloud and the request contains
records the owner has not authorized for cloud, routing refuses the
cloud hop. See `PERMISSION_MODEL.md` for the tool-action declaration and
`RETRIEVAL_ARCHITECTURE.md` for how context is filtered before a provider is
ever called.
