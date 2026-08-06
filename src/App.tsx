import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Sidebar } from "@/components/Sidebar";
import { views, type ViewId } from "@/components/navigation";
import { VaultView } from "@/views/VaultView";
import { ChatView } from "@/views/ChatView";
import { WhatIKnowView } from "@/views/WhatIKnowView";
import { SourcesView } from "@/views/SourcesView";
import { MemoryReviewView } from "@/views/MemoryReviewView";
import { PrivacyAuditView } from "@/views/PrivacyAuditView";
import { BackupRecoveryView } from "@/views/BackupRecoveryView";
import { SettingsView } from "@/views/SettingsView";

interface AppMeta {
  name: string;
  version: string;
  vault_unlocked: boolean;
}

export default function App() {
  const [view, setView] = useState<ViewId>("vaults");
  const [meta, setMeta] = useState<AppMeta | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppMeta>("app_meta")
      .then(setMeta)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="app-shell">
      <Sidebar active={view} onSelect={setView} />
      <main className="app-main">
        <header className="app-header">
          <h1>{views[view].label}</h1>
          <div className="app-meta">
            {error && <span className="badge badge-error">backend error</span>}
            {meta && (
              <>
                <span className="badge">Sammy v{meta.version}</span>
                <span
                  className={"badge " + (meta.vault_unlocked ? "badge-ok" : "badge-warn")}
                  title="Vault lock state"
                >
                  {meta.vault_unlocked ? "vault unlocked" : "vault locked"}
                </span>
              </>
            )}
          </div>
        </header>
        <section className="app-content">{renderView(view)}</section>
      </main>
    </div>
  );
}

function renderView(view: ViewId) {
  switch (view) {
    case "vaults":
      return <VaultView />;
    case "chat":
      return <ChatView />;
    case "whatiknow":
      return <WhatIKnowView />;
    case "sources":
      return <SourcesView />;
    case "memory":
      return <MemoryReviewView />;
    case "privacy":
      return <PrivacyAuditView />;
    case "backup":
      return <BackupRecoveryView />;
    case "settings":
      return <SettingsView />;
  }
}
