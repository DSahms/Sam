import { useCallback, useEffect, useRef, useState } from "react";
import {
  api,
  explainError,
  LOCK_POLICY_LABELS,
  type LockPolicy,
  type VaultSummary,
  type VaultTemplate,
  VAULT_TEMPLATE_LABELS,
} from "@/lib/tauri";

type Mode =
  | { kind: "idle" }
  | { kind: "creating" }
  | { kind: "unlocking"; vaultId: string }
  | { kind: "recovering"; vaultId: string };

function liveValue(ref: { current: HTMLInputElement | null }, fallback: string): string {
  return ref.current?.value ?? fallback;
}

export function VaultView() {
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  const [status, setStatus] = useState<{
    unlocked: boolean;
    active_vault_id: string | null;
  }>({
    unlocked: false,
    active_vault_id: null,
  });
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<Mode>({ kind: "idle" });
  // Create form
  const [name, setName] = useState("");
  const [template, setTemplate] = useState<VaultTemplate>("personal");
  const [passphrase, setPassphrase] = useState("");
  const [policy, setPolicy] = useState<LockPolicy>("minutes_15");
  // Recovery display
  const [recoveryCode, setRecoveryCode] = useState<string | null>(null);
  // Unlock/recovery inputs
  const [unlockPass, setUnlockPass] = useState("");
  const [recoveryInput, setRecoveryInput] = useState("");
  const [recoveryNewPass, setRecoveryNewPass] = useState("");
  const nameRef = useRef<HTMLInputElement>(null);
  const passphraseRef = useRef<HTMLInputElement>(null);
  const unlockPassRef = useRef<HTMLInputElement>(null);
  const recoveryInputRef = useRef<HTMLInputElement>(null);
  const recoveryNewPassRef = useRef<HTMLInputElement>(null);

  const refresh = useCallback(async () => {
    try {
      const [list, st] = await Promise.all([api.vaultList(), api.vaultStatus()]);
      setVaults(list);
      setStatus(st);
      setError(null);
    } catch (e) {
      setError(explainError(e));
    }
  }, []);

  useEffect(() => {
    refresh();
    api
      .lockPolicyGet()
      .then(setPolicy)
      .catch(() => {});
  }, [refresh]);

  const handleCreate = useCallback(async () => {
    setError(null);
    const typedName = liveValue(nameRef, name).trim();
    const typedPass = liveValue(passphraseRef, passphrase);
    if (typedPass.length < 8) {
      setError("Passphrase must be at least 8 characters.");
      return;
    }
    setMode({ kind: "creating" });
    try {
      const result = await api.vaultCreate(typedName || "Personal", template, typedPass);
      setRecoveryCode(result.recovery_code);
      setName("");
      setPassphrase("");
      await refresh();
    } catch (e) {
      setError(explainError(e));
    } finally {
      setMode({ kind: "idle" });
    }
  }, [name, template, passphrase, refresh]);

  const handleUnlock = useCallback(
    async (vaultId: string) => {
      setError(null);
      const typedPass = liveValue(unlockPassRef, unlockPass);
      if (typedPass.length === 0) {
        setError("Passphrase is required.");
        return;
      }
      setMode({ kind: "unlocking", vaultId });
      try {
        await api.vaultUnlock(vaultId, typedPass);
        setUnlockPass("");
        await refresh();
      } catch (e) {
        setError(explainError(e));
      } finally {
        setMode({ kind: "idle" });
      }
    },
    [unlockPass, refresh],
  );

  const handleUnlockRecovery = useCallback(
    async (vaultId: string) => {
      setError(null);
      setMode({ kind: "recovering", vaultId });
      try {
        const typedRecovery = liveValue(recoveryInputRef, recoveryInput);
        const typedNewPass = liveValue(recoveryNewPassRef, recoveryNewPass);
        if (typedNewPass.length < 8) {
          throw new Error("New passphrase must be at least 8 characters.");
        }
        await api.vaultRecoverChangePassphrase(vaultId, typedRecovery, typedNewPass);
        setRecoveryInput("");
        setRecoveryNewPass("");
        await refresh();
      } catch (e) {
        setError(explainError(e));
      } finally {
        setMode({ kind: "idle" });
      }
    },
    [recoveryInput, recoveryNewPass, refresh],
  );

  const handleLock = useCallback(async () => {
    try {
      await api.vaultLock();
      await refresh();
    } catch (e) {
      setError(explainError(e));
    }
  }, [refresh]);

  const handlePolicyChange = useCallback(async (p: LockPolicy) => {
    setPolicy(p);
    try {
      await api.lockPolicySet(p);
    } catch (e) {
      setError(explainError(e));
    }
  }, []);

  return (
    <div className="vault-view">
      <section className="card">
        <h2 className="card-title">Active vault</h2>
        {status.unlocked ? (
          <div className="row">
            <span className="badge badge-ok">
              Unlocked: {status.active_vault_id?.slice(0, 8)}…
            </span>
            <button type="button" className="btn btn-danger" onClick={handleLock}>
              Lock vault
            </button>
          </div>
        ) : (
          <span className="badge badge-warn">Locked — no vault unlocked</span>
        )}
        <div className="row">
          <label htmlFor="policy" className="muted">
            Inactivity lock:
          </label>
          <select
            id="policy"
            value={policy}
            onChange={(e) => handlePolicyChange(e.target.value as LockPolicy)}
            disabled={status.unlocked === undefined}
          >
            {(Object.keys(LOCK_POLICY_LABELS) as LockPolicy[]).map((p) => (
              <option key={p} value={p}>
                {LOCK_POLICY_LABELS[p]}
              </option>
            ))}
          </select>
        </div>
      </section>

      {recoveryCode && (
        <section className="card card-highlight">
          <h2 className="card-title">Recovery code — save this now</h2>
          <p className="muted">
            This is shown <strong>once</strong>. Store it somewhere safe. If you lose both
            your passphrase and this code, the vault is permanently unrecoverable.
          </p>
          <code className="recovery-code">{recoveryCode}</code>
          <div className="row">
            <button
              type="button"
              className="btn"
              onClick={() => navigator.clipboard?.writeText(recoveryCode)}
            >
              Copy
            </button>
            <button type="button" className="btn" onClick={() => setRecoveryCode(null)}>
              I have saved it
            </button>
          </div>
        </section>
      )}

      <section className="card">
        <h2 className="card-title">Create a new vault</h2>
        <div className="form-grid">
          <label htmlFor="vault-name">
            Name
            <input
              id="vault-name"
              ref={nameRef}
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              onInput={(e) => setName((e.target as HTMLInputElement).value)}
              placeholder="Personal"
            />
          </label>
          <label>
            Template
            <select
              value={template}
              onChange={(e) => setTemplate(e.target.value as VaultTemplate)}
            >
              {(Object.keys(VAULT_TEMPLATE_LABELS) as VaultTemplate[]).map((t) => (
                <option key={t} value={t}>
                  {VAULT_TEMPLATE_LABELS[t]}
                </option>
              ))}
            </select>
          </label>
          <label className="span-2" htmlFor="master-passphrase">
            Master passphrase (min 8 characters)
            <input
              id="master-passphrase"
              ref={passphraseRef}
              type="password"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
              onInput={(e) => setPassphrase((e.target as HTMLInputElement).value)}
              autoComplete="new-password"
            />
          </label>
        </div>
        <button
          type="button"
          className="btn btn-primary"
          onClick={handleCreate}
          disabled={mode.kind !== "idle"}
        >
          {mode.kind === "creating" ? "Creating…" : "Create vault"}
        </button>
      </section>

      <section className="card">
        <h2 className="card-title">Vaults on this computer ({vaults.length})</h2>
        {vaults.length === 0 && <p className="muted">No vaults yet. Create one above.</p>}
        <ul className="vault-list">
          {vaults.map((v) => {
            const active = status.active_vault_id === v.vault_id;
            return (
              <li key={v.vault_id} className="vault-item">
                <div className="vault-item-main">
                  <strong>{v.name}</strong>
                  <span className="muted"> · {VAULT_TEMPLATE_LABELS[v.template]}</span>
                  {active && <span className="badge badge-ok">active</span>}
                  <div className="muted small">id: {v.vault_id.slice(0, 13)}…</div>
                </div>
                {!status.unlocked && (
                  <div className="vault-item-actions">
                    <input
                      ref={unlockPassRef}
                      type="password"
                      placeholder="passphrase"
                      aria-label="Vault passphrase"
                      value={unlockPass}
                      onChange={(e) => setUnlockPass(e.target.value)}
                      onInput={(e) => setUnlockPass((e.target as HTMLInputElement).value)}
                      autoComplete="current-password"
                    />
                    <button
                      type="button"
                      className="btn btn-primary"
                      onClick={() => handleUnlock(v.vault_id)}
                      disabled={mode.kind !== "idle"}
                    >
                      Unlock
                    </button>
                    <input
                      ref={recoveryInputRef}
                      type="text"
                      placeholder="recovery code"
                      aria-label="Recovery code"
                      value={recoveryInput}
                      onChange={(e) => setRecoveryInput(e.target.value)}
                      onInput={(e) =>
                        setRecoveryInput((e.target as HTMLInputElement).value)
                      }
                    />
                    <input
                      ref={recoveryNewPassRef}
                      type="password"
                      placeholder="new passphrase"
                      aria-label="New passphrase"
                      value={recoveryNewPass}
                      onChange={(e) => setRecoveryNewPass(e.target.value)}
                      onInput={(e) =>
                        setRecoveryNewPass((e.target as HTMLInputElement).value)
                      }
                      autoComplete="new-password"
                    />
                    <button
                      type="button"
                      className="btn"
                      onClick={() => handleUnlockRecovery(v.vault_id)}
                      disabled={
                        mode.kind !== "idle" ||
                        recoveryInput.length === 0 ||
                        recoveryNewPass.length < 8
                      }
                    >
                      Recover &amp; replace passphrase
                    </button>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      </section>

      {error && (
        <div className="card card-error">
          <strong>Error:</strong> {error}
        </div>
      )}
    </div>
  );
}
