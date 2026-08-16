import { useCallback, useEffect, useRef, useState } from "react";
import {
  api,
  explainError,
  ROUTING_MODE_LABELS,
  type ChatSendResult,
  type ConversationSummary,
  type MessageSummary,
  type RoutingMode,
} from "@/lib/tauri";

export function ChatView() {
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageSummary[]>([]);
  const [input, setInput] = useState("");
  const [routing, setRouting] = useState<RoutingMode>("ask_before_crossing");
  const [model, setModel] = useState("mock-1");
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [providerLabel, setProviderLabel] = useState("mock");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [consent, setConsent] = useState<{
    text: string;
    view: NonNullable<ChatSendResult["consent_required"]>;
  } | null>(null);
  const [crossedToCloud, setCrossedToCloud] = useState(false);
  const messagesEnd = useRef<HTMLDivElement>(null);

  const refreshConversations = useCallback(async () => {
    try {
      const list = await api.conversationList();
      setConversations(list);
      if (!activeId && list.length > 0) {
        setActiveId(list[0].conversation_id);
      }
    } catch (e) {
      setError(explainError(e));
    }
  }, [activeId]);

  const refreshMessages = useCallback(async () => {
    if (!activeId) {
      setMessages([]);
      return;
    }
    try {
      const msgs = await api.conversationMessages(activeId);
      setMessages(msgs);
    } catch (e) {
      setError(explainError(e));
    }
  }, [activeId]);

  useEffect(() => {
    refreshConversations();
  }, [refreshConversations]);

  useEffect(() => {
    refreshMessages();
  }, [refreshMessages]);

  // Load provider config: set the model to the configured default, and if
  // KoboldCpp is enabled, fetch available models for the dropdown.
  useEffect(() => {
    (async () => {
      try {
        const config = await api.providerConfigGet();
        if (config.koboldcpp_enabled && config.koboldcpp_endpoint) {
          setProviderLabel("KoboldCpp (local)");
          if (config.koboldcpp_model) {
            setModel(config.koboldcpp_model);
          }
          // Fetch available models for the dropdown.
          try {
            const models = await api.koboldcppTestConnection(config.koboldcpp_endpoint);
            setAvailableModels(models);
            // If no model was configured, use the first available.
            if (!config.koboldcpp_model && models.length > 0) {
              setModel(models[0]);
            }
          } catch {
            // Endpoint unreachable — leave model as-is; chat_send will
            // fall back to the mock provider.
            setAvailableModels([]);
          }
        } else {
          setProviderLabel("mock (no local provider configured)");
          setModel("mock-1");
        }
      } catch {
        // Config not loadable (e.g. vault locked) — default to mock.
      }
    })();
  }, []);

  useEffect(() => {
    messagesEnd.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const handleNewConversation = useCallback(async () => {
    try {
      const id = await api.conversationCreate("New chat");
      setActiveId(id);
      setMessages([]);
      await refreshConversations();
    } catch (e) {
      setError(explainError(e));
    }
  }, [refreshConversations]);

  const send = useCallback(
    async (text: string, confirmed: boolean) => {
      if (!activeId || !text.trim()) return;
      setBusy(true);
      setError(null);
      setConsent(null);
      try {
        const result = await api.chatSend(activeId, text, routing, model, confirmed);
        if (result.consent_required) {
          setConsent({ text, view: result.consent_required });
          return;
        }
        setCrossedToCloud(result.crossed_to_cloud);
        setInput("");
        await refreshMessages();
      } catch (e) {
        setError(explainError(e));
      } finally {
        setBusy(false);
      }
    },
    [activeId, routing, model, refreshMessages],
  );

  const handleSend = useCallback(() => {
    void send(input, false);
  }, [input, send]);

  const handleConfirmConsent = useCallback(() => {
    if (consent) {
      void send(consent.text, true);
    }
  }, [consent, send]);

  const handleDenyConsent = useCallback(() => {
    setConsent(null);
    setError("Cloud crossing denied — no content was transmitted.");
  }, []);

  return (
    <div className="chat-view">
      <aside className="chat-sidebar">
        <button type="button" className="btn btn-primary" onClick={handleNewConversation}>
          + New chat
        </button>
        <ul className="conv-list">
          {conversations.map((c) => (
            <li key={c.conversation_id}>
              <button
                type="button"
                className={
                  "conv-item" + (c.conversation_id === activeId ? " active" : "")
                }
                onClick={() => setActiveId(c.conversation_id)}
              >
                {c.title || "Untitled"}
              </button>
            </li>
          ))}
          {conversations.length === 0 && (
            <li className="muted small">No conversations yet.</li>
          )}
        </ul>
      </aside>

      <section className="chat-main">
        <div className="chat-toolbar">
          <label className="muted small">
            Routing:
            <select
              value={routing}
              onChange={(e) => setRouting(e.target.value as RoutingMode)}
            >
              {(Object.keys(ROUTING_MODE_LABELS) as RoutingMode[]).map((r) => (
                <option key={r} value={r}>
                  {ROUTING_MODE_LABELS[r]}
                </option>
              ))}
            </select>
          </label>
          <label className="muted small">
            Model:
            {availableModels.length > 0 ? (
              <select value={model} onChange={(e) => setModel(e.target.value)}>
                {availableModels.map((m) => (
                  <option key={m} value={m}>
                    {m}
                  </option>
                ))}
              </select>
            ) : (
              <input value={model} onChange={(e) => setModel(e.target.value)} />
            )}
          </label>
          <span className="badge" title="Active provider">
            {providerLabel}
          </span>
          {crossedToCloud && (
            <span
              className="badge badge-warn"
              title="Last turn crossed to a cloud provider"
            >
              crossed to cloud
            </span>
          )}
        </div>

        <div className="chat-messages">
          {messages.length === 0 && (
            <p className="muted">
              No messages yet. Send a message to start chatting with Sammy through the
              mock provider.
            </p>
          )}
          {messages.map((m) => (
            <div key={m.message_id} className={"msg msg-" + m.role}>
              <div className="msg-role">{m.role}</div>
              <div className="msg-content">{m.content}</div>
              {m.role === "assistant" && m.pkc?.used && (
                <details
                  className="pkc-provenance"
                  onToggle={(e) => {
                    if ((e.target as HTMLDetailsElement).open) {
                      void api.pkcProvenanceOpened(m.message_id);
                    }
                  }}
                >
                  <summary>Used personal knowledge</summary>
                  <p className="muted small">
                    Sammy consulted authorized personal knowledge for this answer. It was
                    not copied into Sammy&apos;s lasting memory.
                  </p>
                  <ul className="muted small pkc-provenance-meta">
                    {m.pkc.classification && (
                      <li>Kind: stored source-backed knowledge</li>
                    )}
                    {m.pkc.source_id && <li>Source: {m.pkc.source_id}</li>}
                    {m.pkc.payload_chars > 0 && (
                      <li>Evidence size: {m.pkc.payload_chars} characters</li>
                    )}
                  </ul>
                </details>
              )}
              {m.role === "assistant" && !m.pkc?.used && m.pkc?.owner_notice && (
                <div className="muted small pkc-notice">{m.pkc.owner_notice}</div>
              )}
            </div>
          ))}
          <div ref={messagesEnd} />
        </div>

        {consent && (
          <div className="card card-highlight consent-banner">
            <strong>Cloud crossing requested</strong>
            <p className="muted small">
              Sending to <strong>{consent.view.provider_display_name}</strong> (
              {consent.view.model}) means the conversation content leaves this device.
              Routing mode: {consent.view.routing_mode}. Approve?
            </p>
            <div className="row">
              <button
                type="button"
                className="btn btn-primary"
                onClick={handleConfirmConsent}
              >
                Approve and send
              </button>
              <button
                type="button"
                className="btn btn-danger"
                onClick={handleDenyConsent}
              >
                Deny
              </button>
            </div>
          </div>
        )}

        <div className="chat-input-row">
          <textarea
            className="chat-input"
            placeholder="Message Sammy…"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            rows={2}
          />
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleSend}
            disabled={busy || !input.trim() || !activeId}
          >
            {busy ? "…" : "Send"}
          </button>
        </div>

        {error && (
          <div className="card card-error">
            <strong>Error:</strong> {error}
          </div>
        )}
      </section>
    </div>
  );
}
