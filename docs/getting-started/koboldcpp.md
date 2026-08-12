# KoboldCpp setup

[Quick start](quick-start.md) · [Providers](../user-guide/providers.md) · [Troubleshooting](../TROUBLESHOOTING.md)

Sammy integrates with a KoboldCpp OpenAI-compatible HTTP API and was validated at
`http://localhost:5001` with model discovery and a real chat response.

1. Start KoboldCpp with its compatible API enabled and a model loaded.
2. Unlock a vault and open **Settings**.
3. Set the endpoint and select **Test connection**. Sammy calls `/v1/models`.
4. Choose the model, enable KoboldCpp, save, and verify the badge in **Chat**.

Chat uses `/v1/chat/completions`, a 120-second timeout, and normalizes trailing
slashes. A successful local result has `crossed_to_cloud: false` and a local
provider audit event.

Common failures are connection refusal, a mismatched port, no loaded model, and
inference exceeding the timeout. A deterministic non-cloud mock remains a
development fallback; Sammy does not convert a failed local request into an
unconfirmed Venice send.
