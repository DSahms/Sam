import { useCallback, useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, explainError, type VaultStatus } from "@/lib/tauri";

interface BackupPreview {
  format_version: number;
  vault_id: string;
  vault_name: string;
  created_at: string;
}

type RestoreStep = "idle" | "selected" | "confirmed";

export function BackupRecoveryView() {
  const [status, setStatus] = useState<VaultStatus>({
    unlocked: false,
    active_vault_id: null,
  });
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // --- Backup form state ---
  const [backupPass, setBackupPass] = useState("");
  const [backupRecovery, setBackupRecovery] = useState("");
  const [backupPath, setBackupPath] = useState<string | null>(null);
  const [backupBusy, setBackupBusy] = useState(false);

  // --- Restore state ---
  const [restoreStep, setRestoreStep] = useState<RestoreStep>("idle");
  const [restorePath, setRestorePath] = useState<string | null>(null);
  const [restorePreview, setRestorePreview] = useState<BackupPreview | null>(null);
  const [restorePass, setRestorePass] = useState("");
  const [restoreBusy, setRestoreBusy] = useState(false);
  const [restoreResult, setRestoreResult] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const st = await api.vaultStatus();
      setStatus(st);
      setError(null);
    } catch (e) {
      setError(explainError(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Clear success/error after a while
  useEffect(() => {
    if (success) {
      const t = setTimeout(() => setSuccess(null), 5000);
      return () => clearTimeout(t);
    }
  }, [success]);

  // --- Backup handlers ---
  const handleChooseBackupDest = useCallback(async () => {
    try {
      const selected = await save({
        title: "Choose where to save the backup",
        defaultPath: "sammy-backup.sammy-backup",
        filters: [{ name: "Sammy Backup", extensions: ["sammy-backup"] }],
      });
      if (selected) setBackupPath(selected);
    } catch (e) {
      setError(explainError(e));
    }
  }, []);

  const handleBackup = useCallback(async () => {
    if (!status.active_vault_id || !backupPath) return;
    if (backupPass.length < 8) {
      setError("Passphrase must be at least 8 characters.");
      return;
    }
    if (!backupRecovery.trim()) {
      setError("Recovery code is required to create a backup.");
      return;
    }
    setBackupBusy(true);
    setError(null);
    setSuccess(null);
    try {
      await api.vaultBackup(
        status.active_vault_id,
        backupPass,
        backupRecovery.trim(),
        backupPath,
      );
      setSuccess(`Backup created successfully at ${backupPath}`);
      setBackupPass("");
      setBackupRecovery("");
      setBackupPath(null);
    } catch (e) {
      setError(explainError(e));
    } finally {
      setBackupBusy(false);
    }
  }, [status.active_vault_id, backupPath, backupPass, backupRecovery]);

  // --- Restore handlers ---
  const handleChooseRestoreFile = useCallback(async () => {
    try {
      const selected = await open({
        title: "Select a Sammy backup file to restore",
        multiple: false,
        filters: [{ name: "Sammy Backup", extensions: ["sammy-backup"] }],
      });
      if (!selected) return;
      const path = selected as string;
      setRestorePath(path);
      setRestoreStep("selected");
      setRestorePreview(null);
      setRestoreResult(null);
      setError(null);
      // Preview the backup header (no credentials needed).
      try {
        const preview = await api.vaultBackupPreview(path);
        setRestorePreview(preview);
      } catch (e) {
        setError(`This does not appear to be a valid Sammy backup: ${explainError(e)}`);
        setRestoreStep("idle");
        setRestorePath(null);
      }
    } catch (e) {
      setError(explainError(e));
    }
  }, []);

  const handleConfirmRestore = useCallback(() => {
    setRestoreStep("confirmed");
  }, []);

  const handleExecuteRestore = useCallback(async () => {
    if (!restorePath) return;
    if (restorePass.length < 8) {
      setError("Passphrase must be at least 8 characters.");
      return;
    }
    setRestoreBusy(true);
    setError(null);
    setRestoreResult(null);
    try {
      const restoredId = await api.vaultRestorePassphrase(restorePath, restorePass);
      setRestoreResult(
        `Backup restored successfully. Vault ${restoredId.slice(0, 8)}… is now available. Lock and re-unlock to use it.`,
      );
      setRestoreStep("idle");
      setRestorePath(null);
      setRestorePreview(null);
      setRestorePass("");
      await refresh();
    } catch (e) {
      setError(
        `Restore failed: ${explainError(e)}. If the passphrase is wrong, the backup cannot be decrypted.`,
      );
    } finally {
      setRestoreBusy(false);
    }
  }, [restorePath, restorePass, refresh]);

  const handleCancelRestore = useCallback(() => {
    setRestoreStep("idle");
    setRestorePath(null);
    setRestorePreview(null);
    setRestorePass("");
    setError(null);
  }, []);

  return (
    <div className="backup-view">
      {/* --- Backup section --- */}
      <section className="card">
        <h2 className="card-title">Create Backup</h2>
        <p className="muted small">
          Backups encrypt your entire vault — sources, knowledge, conversations, and
          identity — into a single file you can save anywhere. The file can only be opened
          with your passphrase or recovery code.
        </p>
        {!status.unlocked && (
          <p className="badge badge-warn">A vault must be unlocked to create a backup.</p>
        )}
        <div className="form-grid">
          <label className="span-2">
            Master passphrase
            <input
              type="password"
              value={backupPass}
              onChange={(e) => setBackupPass(e.target.value)}
              placeholder="Your vault passphrase"
              autoComplete="current-password"
              disabled={!status.unlocked}
            />
          </label>
          <label className="span-2">
            Recovery code
            <input
              type="text"
              value={backupRecovery}
              onChange={(e) => setBackupRecovery(e.target.value)}
              placeholder="Your recovery code (e.g. K5QX-7M2J-…)"
              disabled={!status.unlocked}
            />
          </label>
        </div>
        <div className="row">
          <button
            type="button"
            className="btn"
            onClick={handleChooseBackupDest}
            disabled={!status.unlocked}
          >
            {backupPath ? `✓ ${backupPath}` : "Choose destination…"}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleBackup}
            disabled={
              backupBusy ||
              !status.unlocked ||
              !backupPath ||
              backupPass.length < 8 ||
              !backupRecovery.trim()
            }
          >
            {backupBusy ? "Creating backup…" : "Create Backup"}
          </button>
        </div>
      </section>

      {/* --- Restore section --- */}
      <section className="card">
        <h2 className="card-title">Restore from Backup</h2>
        <p className="muted small">
          Restore a vault from a previously created backup file. You will need the
          passphrase or recovery code that was used when the backup was created.
        </p>

        {restoreStep === "idle" && (
          <button type="button" className="btn" onClick={handleChooseRestoreFile}>
            Select backup file…
          </button>
        )}

        {restoreStep !== "idle" && restorePath && (
          <div className="restore-flow">
            <div className="card" style={{ marginBottom: "12px" }}>
              <strong>Selected backup:</strong>
              <div className="muted small">{restorePath}</div>
              {restorePreview && (
                <div style={{ marginTop: "8px" }}>
                  <div>
                    <strong>{restorePreview.vault_name}</strong> (format v
                    {restorePreview.format_version})
                  </div>
                  <div className="muted small">
                    Vault ID: {restorePreview.vault_id.slice(0, 13)}…
                  </div>
                  <div className="muted small">
                    Created: {restorePreview.created_at.slice(0, 19)}
                  </div>
                </div>
              )}
            </div>

            {restoreStep === "selected" && (
              <>
                <div className="card card-highlight" style={{ marginBottom: "12px" }}>
                  <strong>⚠ Warning</strong>
                  <p className="muted small" style={{ margin: "6px 0" }}>
                    Restoring this backup will <strong>replace</strong> any existing vault
                    data with the same vault ID. This action cannot be undone. Make sure
                    you have selected the correct backup file.
                  </p>
                </div>
                <div className="row">
                  <button
                    type="button"
                    className="btn btn-danger"
                    onClick={handleConfirmRestore}
                  >
                    I understand — continue
                  </button>
                  <button type="button" className="btn" onClick={handleCancelRestore}>
                    Cancel
                  </button>
                </div>
              </>
            )}

            {restoreStep === "confirmed" && (
              <>
                <label
                  className="span-2"
                  style={{ display: "flex", flexDirection: "column", gap: "4px" }}
                >
                  Passphrase for this backup
                  <input
                    type="password"
                    value={restorePass}
                    onChange={(e) => setRestorePass(e.target.value)}
                    placeholder="The passphrase used when this backup was created"
                    autoComplete="current-password"
                  />
                </label>
                <div className="row" style={{ marginTop: "12px" }}>
                  <button
                    type="button"
                    className="btn btn-danger"
                    onClick={handleExecuteRestore}
                    disabled={restoreBusy || restorePass.length < 8}
                  >
                    {restoreBusy ? "Restoring…" : "Restore Backup"}
                  </button>
                  <button type="button" className="btn" onClick={handleCancelRestore}>
                    Cancel
                  </button>
                </div>
              </>
            )}
          </div>
        )}

        {restoreResult && (
          <div className="card" style={{ borderColor: "var(--ok)", marginTop: "12px" }}>
            <span style={{ color: "var(--ok)" }}>✓ {restoreResult}</span>
          </div>
        )}
      </section>

      {success && (
        <div className="card" style={{ borderColor: "var(--ok)" }}>
          <span style={{ color: "var(--ok)" }}>✓ {success}</span>
        </div>
      )}

      {error && (
        <div className="card card-error">
          <strong>Error:</strong> {error}
        </div>
      )}
    </div>
  );
}
