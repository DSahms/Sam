import { useCallback, useEffect, useState } from "react";
import { api, explainError, type ProviderConfig } from "@/lib/tauri";

export function SettingsView() {
  const [config, setConfig] = useState<ProviderConfig>({
    koboldcpp_endpoint: "",
    koboldcpp_model: "",
    koboldcpp_enabled: false,
    venice_endpoint: "https://api.venice.ai/api/v1",
    venice_model: "",
    venice_enabled: false,
    venice_has_api_key: false,
  });
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    ok: boolean;
    models?: string[];
    msg: string;
  } | null>(null);
  const [chatTest, setChatTest] = useState<string | null>(null);
  const [chatBusy, setChatBusy] = useState(false);
  const [veniceKey, setVeniceKey] = useState("");
  const [veniceTesting, setVeniceTesting] = useState(false);
  const [veniceResult, setVeniceResult] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const c = await api.providerConfigGet();
      setConfig(c);
      setError(null);
    } catch (e) {
      setError(explainError(e));
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleSave = useCallback(async () => {
    setError(null);
    try {
      await api.providerConfigSave(config);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(explainError(e));
    }
  }, [config]);

  const handleTest = useCallback(async () => {
    setTesting(true);
    setTestResult(null);
    setError(null);
    try {
      const models = await api.koboldcppTestConnection(config.koboldcpp_endpoint);
      setTestResult({
        ok: true,
        models,
        msg: `Connected. ${models.length} model(s) available.`,
      });
      // Auto-fill the model field if empty.
      if (!config.koboldcpp_model && models.length > 0) {
        setConfig({ ...config, koboldcpp_model: models[0] });
      }
    } catch (e) {
      setTestResult({ ok: false, msg: explainError(e) });
    } finally {
      setTesting(false);
    }
  }, [config]);

  const handleChatTest = useCallback(async () => {
    setChatBusy(true);
    setChatTest(null);
    setError(null);
    try {
      const result = await api.koboldcppChat(
        config.koboldcpp_endpoint,
        config.koboldcpp_model,
        "You are Sammy, a helpful assistant. Reply briefly.",
        "Hello! Who are you?",
        128,
      );
      setChatTest(result.content);
    } catch (e) {
      setError(explainError(e));
    } finally {
      setChatBusy(false);
    }
  }, [config]);

  const handleSaveVenice = useCallback(async () => {
    setError(null);
    setVeniceResult(null);
    try {
      await api.providerConfigSave(config);
      if (veniceKey) {
        await api.veniceApiKeySet(veniceKey);
        setVeniceKey("");
        setConfig((current) => ({ ...current, venice_has_api_key: true }));
      }
      setVeniceResult("Venice settings saved.");
    } catch (e) {
      setError(explainError(e));
    }
  }, [config, veniceKey]);

  const handleTestVenice = useCallback(async () => {
    setVeniceTesting(true);
    setVeniceResult(null);
    setError(null);
    try {
      await api.providerConfigSave(config);
      if (veniceKey) {
        await api.veniceApiKeySet(veniceKey);
        setVeniceKey("");
        setConfig((current) => ({ ...current, venice_has_api_key: true }));
      }
      const models = await api.veniceTestConnection();
      setVeniceResult(`Connected securely. ${models.length} model(s) available.`);
      if (!config.venice_model && models[0]) {
        setConfig((current) => ({ ...current, venice_model: models[0] }));
      }
    } catch (e) {
      setError(explainError(e));
    } finally {
      setVeniceTesting(false);
    }
  }, [config, veniceKey]);

  return (
    <div className="settings-view">
      <section className="card">
        <h2 className="card-title">KoboldCpp (local provider)</h2>
        <p className="muted small">
          Configure a local KoboldCpp-compatible endpoint. All communication stays on this
          machine — local-only, no cloud crossing.
        </p>
        <div className="form-grid">
          <label className="span-2">
            Endpoint URL
            <input
              type="text"
              value={config.koboldcpp_endpoint}
              onChange={(e) =>
                setConfig({ ...config, koboldcpp_endpoint: e.target.value })
              }
              placeholder="http://localhost:5001"
            />
          </label>
          <label>
            Default model
            <input
              type="text"
              value={config.koboldcpp_model}
              onChange={(e) => setConfig({ ...config, koboldcpp_model: e.target.value })}
              placeholder="(auto-detect on test)"
            />
          </label>
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={config.koboldcpp_enabled}
              onChange={(e) =>
                setConfig({ ...config, koboldcpp_enabled: e.target.checked })
              }
            />
            Enabled
          </label>
        </div>
        <div className="row">
          <button type="button" className="btn btn-primary" onClick={handleSave}>
            {saved ? "Saved ✓" : "Save settings"}
          </button>
          <button
            type="button"
            className="btn"
            onClick={handleTest}
            disabled={testing || !config.koboldcpp_endpoint}
          >
            {testing ? "Testing…" : "Test connection"}
          </button>
          {testResult && (
            <span className={testResult.ok ? "badge badge-ok" : "badge badge-error"}>
              {testResult.msg}
            </span>
          )}
        </div>
        {testResult?.models && testResult.models.length > 0 && (
          <div className="muted small">
            Available models: {testResult.models.join(", ")}
          </div>
        )}
      </section>

      {testResult?.ok && (
        <section className="card">
          <h2 className="card-title">Live chat test</h2>
          <p className="muted small">Send a test message to verify the model responds.</p>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleChatTest}
            disabled={chatBusy || !config.koboldcpp_model}
          >
            {chatBusy ? "Waiting for model…" : 'Send "Hello! Who are you?"'}
          </button>
          {chatTest && (
            <div className="card" style={{ marginTop: "12px" }}>
              <div className="muted small">Model response:</div>
              <div className="msg-content" style={{ marginTop: "6px" }}>
                {chatTest}
              </div>
            </div>
          )}
        </section>
      )}

      <section className="card">
        <h2 className="card-title">Venice (cloud provider)</h2>
        <p className="muted small">
          Cloud use is always governed by the routing and consent controls in Chat. The
          API key is encrypted inside the active vault and is never displayed again.
        </p>
        <div className="form-grid">
          <label className="span-2">
            Endpoint
            <input type="text" value={config.venice_endpoint} readOnly />
          </label>
          <label>
            Default model
            <input
              type="text"
              value={config.venice_model}
              onChange={(e) => setConfig({ ...config, venice_model: e.target.value })}
              placeholder="Select after testing"
            />
          </label>
          <label>
            API key{" "}
            {config.venice_has_api_key && <span className="badge badge-ok">stored</span>}
            <input
              type="password"
              value={veniceKey}
              onChange={(e) => setVeniceKey(e.target.value)}
              placeholder={
                config.venice_has_api_key ? "Enter to replace stored key" : "Required"
              }
              autoComplete="new-password"
            />
          </label>
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={config.venice_enabled}
              onChange={(e) => setConfig({ ...config, venice_enabled: e.target.checked })}
            />
            Enabled
          </label>
        </div>
        <div className="row">
          <button type="button" className="btn btn-primary" onClick={handleSaveVenice}>
            Save Venice settings
          </button>
          <button
            type="button"
            className="btn"
            onClick={handleTestVenice}
            disabled={veniceTesting || (!veniceKey && !config.venice_has_api_key)}
          >
            {veniceTesting ? "Testing…" : "Test secure connection"}
          </button>
          {veniceResult && <span className="badge badge-ok">{veniceResult}</span>}
        </div>
      </section>

      {error && (
        <div className="card card-error">
          <strong>Error:</strong> {error}
        </div>
      )}
    </div>
  );
}
