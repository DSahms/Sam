import { useCallback, useEffect, useState } from "react";
import {
  api,
  explainError,
  type PkcHealthReport,
  type ProviderConfig,
} from "@/lib/tauri";

function healthBadgeClass(state: string, probed: boolean): string {
  if (state === "available_authorized" && probed) return "badge badge-ok";
  if (
    state === "disabled" ||
    state === "retrieval_skipped" ||
    state === "configured_not_tested"
  ) {
    return "badge";
  }
  if (state === "connecting") return "badge badge-warn";
  return "badge badge-error";
}

function healthLabel(state: string): string {
  switch (state) {
    case "disabled":
      return "Off";
    case "configured_not_tested":
      return "Not tested";
    case "connecting":
      return "Checking…";
    case "available_authorized":
      return "Authorized";
    case "available_unauthorized":
      return "Unauthorized";
    case "pkc_unavailable":
      return "Unavailable";
    case "bridge_unavailable":
      return "Bridge missing";
    case "python_unavailable":
      return "Python missing";
    case "misconfigured":
      return "Misconfigured";
    case "local_model_unavailable":
      return "Local model unavailable";
    case "cloud_turn_skipped":
      return "Skipped on cloud turns";
    default:
      return state;
  }
}

export function SettingsView() {
  const [config, setConfig] = useState<ProviderConfig>({
    koboldcpp_endpoint: "",
    koboldcpp_model: "",
    koboldcpp_enabled: false,
    venice_endpoint: "https://api.venice.ai/api/v1",
    venice_model: "",
    venice_enabled: false,
    venice_has_api_key: false,
    pkc_enabled: false,
    pkc_python_executable: "",
    pkc_bridge_script: "",
    pkc_root: "",
    pkc_source_id: "",
    pkc_last_health_state: "",
    pkc_last_health_at: "",
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
  const [pkcHealth, setPkcHealth] = useState<PkcHealthReport | null>(null);
  const [pkcBusy, setPkcBusy] = useState(false);
  const [pkcSaved, setPkcSaved] = useState(false);

  const load = useCallback(async () => {
    try {
      const c = await api.providerConfigGet();
      setConfig(c);
      setError(null);
      try {
        const health = await api.pkcHealthCheck(c, false);
        setPkcHealth(health);
      } catch {
        setPkcHealth(null);
      }
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

  const handleSavePkc = useCallback(async () => {
    setError(null);
    try {
      await api.providerConfigSave(config);
      const refreshed = await api.providerConfigGet();
      setConfig(refreshed);
      setPkcSaved(true);
      setTimeout(() => setPkcSaved(false), 2000);
      const health = await api.pkcHealthCheck(refreshed, false);
      setPkcHealth(health);
    } catch (e) {
      setError(explainError(e));
    }
  }, [config]);

  const handleDiscoverPkc = useCallback(async () => {
    setError(null);
    try {
      const found = await api.pkcDiscoverDefaults();
      setConfig((current) => ({
        ...current,
        pkc_python_executable: current.pkc_python_executable || found.python_executable,
        pkc_bridge_script: current.pkc_bridge_script || found.bridge_script,
        pkc_root: current.pkc_root || found.pkc_root,
        pkc_source_id: current.pkc_source_id || found.source_id,
      }));
      if (found.notes.length > 0 && !found.bridge_script && !found.pkc_root) {
        setError(found.notes.join(" "));
      }
    } catch (e) {
      setError(explainError(e));
    }
  }, []);

  const handleTestPkc = useCallback(async () => {
    setPkcBusy(true);
    setError(null);
    try {
      await api.providerConfigSave(config);
      const connecting: PkcHealthReport = {
        state: "connecting",
        owner_message: "Checking the personal-knowledge connection…",
        consumer: "sammy",
        purpose: "personal_consigliere",
        local_only: true,
        python_ok: false,
        bridge_ok: false,
        root_ok: false,
        source_configured: !!config.pkc_source_id,
        authorized: null,
        probed: false,
        payload_chars: 0,
        payload_sha256: null,
      };
      setPkcHealth(connecting);
      const health = await api.pkcHealthCheck(config, true);
      setPkcHealth(health);
      const refreshed = await api.providerConfigGet();
      setConfig(refreshed);
    } catch (e) {
      setError(explainError(e));
    } finally {
      setPkcBusy(false);
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

      <section className="card">
        <h2 className="card-title">Personal knowledge (PKC)</h2>
        <p className="muted small">
          Optional read-only access to the separate Personal Knowledge Corpus product —
          not Sammy&apos;s encrypted vault. Off by default. Sammy never writes to PKC,
          never copies it into lasting Sammy memory, and never sends it to a cloud model.
        </p>
        <div className="row" style={{ marginBottom: "12px" }}>
          <span
            className={healthBadgeClass(
              pkcHealth?.state ||
                (config.pkc_enabled ? "configured_not_tested" : "disabled"),
              !!pkcHealth?.probed ||
                config.pkc_last_health_state === "available_authorized",
            )}
          >
            {pkcBusy
              ? "Checking…"
              : healthLabel(
                  pkcHealth?.state ||
                    config.pkc_last_health_state ||
                    (config.pkc_enabled ? "configured_not_tested" : "disabled"),
                )}
          </span>
          {config.pkc_last_health_at && (
            <span className="muted small">Last checked {config.pkc_last_health_at}</span>
          )}
        </div>
        <p className="muted small">
          {pkcHealth?.owner_message ||
            (config.pkc_enabled
              ? "Use Test connection to verify authorization before relying on it in chat."
              : "Enable this only if you want Sammy to consult your durable personal knowledge on local turns.")}
        </p>
        <div className="form-grid">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={config.pkc_enabled}
              onChange={(e) => setConfig({ ...config, pkc_enabled: e.target.checked })}
            />
            Enable read-only personal knowledge
          </label>
          <label>
            Consumer
            <input type="text" value="sammy" readOnly />
          </label>
          <label>
            Purpose
            <input type="text" value="personal_consigliere" readOnly />
          </label>
          <label className="span-2">
            PKC location
            <input
              type="text"
              value={config.pkc_root}
              onChange={(e) => setConfig({ ...config, pkc_root: e.target.value })}
              placeholder="Folder containing the Personal Knowledge Corpus"
            />
          </label>
          <label className="span-2">
            Source identity
            <input
              type="text"
              value={config.pkc_source_id}
              onChange={(e) => setConfig({ ...config, pkc_source_id: e.target.value })}
              placeholder="Discovered or pasted source ID"
            />
          </label>
        </div>
        <p className="muted small">
          Local-only: eligible local chats may consult PKC. Cloud chats skip it entirely.
        </p>
        <details className="pkc-advanced">
          <summary>Advanced connection details</summary>
          <div className="form-grid" style={{ marginTop: "8px" }}>
            <label>
              Python
              <input
                type="text"
                value={config.pkc_python_executable}
                onChange={(e) =>
                  setConfig({ ...config, pkc_python_executable: e.target.value })
                }
                placeholder="python"
              />
            </label>
            <label className="span-2">
              Bridge script
              <input
                type="text"
                value={config.pkc_bridge_script}
                onChange={(e) =>
                  setConfig({ ...config, pkc_bridge_script: e.target.value })
                }
                placeholder="Absolute path to the PKC bridge"
              />
            </label>
          </div>
        </details>
        <div className="row">
          <button type="button" className="btn" onClick={() => void handleDiscoverPkc()}>
            Find local defaults
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => void handleSavePkc()}
          >
            {pkcSaved ? "Saved ✓" : "Save"}
          </button>
          <button
            type="button"
            className="btn"
            onClick={() => void handleTestPkc()}
            disabled={pkcBusy}
          >
            {pkcBusy ? "Testing…" : "Test connection"}
          </button>
        </div>
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
