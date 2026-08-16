# Chat

[Documentation home](../README.md) · [Providers](providers.md) · [Knowledge corpus](knowledge-corpus.md)

Chat stores conversations and messages in the active encrypted vault. Create a
conversation, select a routing mode and model, inspect the provider badge, and
send. The backend assembles identity plus retrieved current knowledge, applies
privacy filters, routes the request, calls the provider, and records the result.

Cloud routing may display a consent banner. Approving permits that invocation;
denial records no transmission. `local_only` knowledge is removed from
cloud-bound context. Provider errors are sanitized and do not include the private
prompt.

Chat is currently request/response rather than token-streaming. If no real local
adapter is usable, a deterministic mock can keep development workflows operable;
it is not a cloud fallback. A provider outage does not erase the conversation or
corpus.

When personal knowledge (PKC) is enabled and the turn stays local, Sammy may
consult that separate corpus for questions about you. A **Used personal
knowledge** control appears on those answers so you can see that stored
knowledge was used, without dumping private source text into the thread.
Cloud routing never consults PKC. See
[external PKC](../getting-started/external-pkc.md).
