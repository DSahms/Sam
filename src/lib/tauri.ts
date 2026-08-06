// Typed wrappers around the Sammy Tauri commands. Each function invokes the
// corresponding Rust command and returns typed data, throwing on the
// sanitized {kind, message} error shape returned by AppError::serialize.

import { invoke } from "@tauri-apps/api/core";

export type VaultTemplate = "personal" | "witness" | "consigliere" | "custom";

export interface VaultSummary {
  vault_id: string;
  name: string;
  template: VaultTemplate;
  created_at: string;
}

export interface CreateVaultResult {
  vault: VaultSummary;
  /** Human-form recovery code. Shown exactly once; never stored. */
  recovery_code: string;
}

export interface VaultStatus {
  unlocked: boolean;
  active_vault_id: string | null;
}

export interface AppErrorShape {
  kind: string;
  message: string;
}

function isAppError(e: unknown): e is AppErrorShape {
  return (
    typeof e === "object" &&
    e !== null &&
    "kind" in e &&
    "message" in e &&
    typeof (e as { kind: unknown }).kind === "string"
  );
}

/** Reduce a thrown command error to a human string. */
export function explainError(e: unknown): string {
  if (isAppError(e)) {
    return e.message;
  }
  if (e instanceof Error) return e.message;
  return String(e);
}

export const api = {
  ping: () => invoke<string>("ping"),
  appMeta: () =>
    invoke<{ name: string; version: string; vault_unlocked: boolean }>("app_meta"),

  vaultList: () => invoke<VaultSummary[]>("vault_list"),
  vaultCreate: (name: string, template: VaultTemplate, passphrase: string) =>
    invoke<CreateVaultResult>("vault_create", { name, template, passphrase }),
  vaultUnlock: (vaultId: string, passphrase: string) =>
    invoke<VaultSummary>("vault_unlock", { vaultId, passphrase }),
  vaultUnlockRecovery: (vaultId: string, recoveryCode: string) =>
    invoke<VaultSummary>("vault_unlock_recovery", { vaultId, recoveryCode }),
  vaultLock: () => invoke<void>("vault_lock"),
  vaultStatus: () => invoke<VaultStatus>("vault_status"),
  touchActivity: () => invoke<void>("touch_activity"),
  lockPolicyGet: () => invoke<LockPolicy>("lock_policy_get"),
  lockPolicySet: (policy: LockPolicy) => invoke<void>("lock_policy_set", { policy }),
};

export type LockPolicy =
  "minutes_5" | "minutes_15" | "minutes_30" | "minutes_60" | "manual_only";

export const LOCK_POLICY_LABELS: Record<LockPolicy, string> = {
  minutes_5: "5 minutes",
  minutes_15: "15 minutes (default)",
  minutes_30: "30 minutes",
  minutes_60: "60 minutes",
  manual_only: "Manual only",
};

export const VAULT_TEMPLATE_LABELS: Record<VaultTemplate, string> = {
  personal: "Personal",
  witness: "Witness",
  consigliere: "Consigliere",
  custom: "Custom",
};
