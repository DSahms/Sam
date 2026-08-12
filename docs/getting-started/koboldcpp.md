# KoboldCpp setup

[Quick start](quick-start.md) · [Providers](../user-guide/providers.md) · [Troubleshooting](../TROUBLESHOOTING.md)

Sammy integrates with a KoboldCpp OpenAI-compatible HTTP API and was validated at
`http://localhost:5001` with model discovery and a real chat response.

An **API** is a structured way for two programs to communicate. The **endpoint**
is its address. Here, `localhost` means your own computer and port `5001`
identifies the KoboldCpp service. No Internet connection is involved when both
programs run locally.

Install and operate KoboldCpp according to its own trusted distribution
instructions, load a compatible model, and enable its OpenAI-compatible API.
Sammy does not download models or start KoboldCpp for you.

1. Start KoboldCpp with its compatible API enabled and a model loaded.
2. Unlock a vault and open **Settings**.
3. Set the endpoint and select **Test connection**. Sammy calls `/v1/models`.
4. Choose the model, enable KoboldCpp, save, and verify the badge in **Chat**.

The connection test succeeds when Settings displays one or more model IDs. A
normal Chat test succeeds when a response appears and the request is recorded as
local rather than cloud-crossing.

Chat uses `/v1/chat/completions`, a 120-second timeout, and normalizes trailing
slashes. A successful local result has `crossed_to_cloud: false` and a local
provider audit event.

Common failures are connection refusal, a mismatched port, no loaded model, and
inference exceeding the timeout. A deterministic non-cloud mock remains a
development fallback; Sammy does not convert a failed local request into an
unconfirmed Venice send.
