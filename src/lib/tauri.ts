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
  vaultBackup: (
    vaultId: string,
    passphrase: string,
    recoveryCode: string,
    outPath: string,
  ) => invoke<void>("vault_backup", { vaultId, passphrase, recoveryCode, outPath }),
  vaultRestorePassphrase: (inPath: string, passphrase: string, confirmed: boolean) =>
    invoke<string>("vault_restore_passphrase", { inPath, passphrase, confirmed }),
  vaultRestoreRecovery: (inPath: string, recoveryCode: string, confirmed: boolean) =>
    invoke<string>("vault_restore_recovery", { inPath, recoveryCode, confirmed }),
  vaultBackupPreview: (inPath: string) =>
    invoke<{
      format_version: number;
      vault_id: string;
      vault_name: string;
      created_at: string;
    }>("vault_backup_preview", { inPath }),

  // Conversations + chat
  conversationList: () => invoke<ConversationSummary[]>("conversation_list"),
  conversationCreate: (title: string | null) =>
    invoke<string>("conversation_create", { title: title ?? null }),
  conversationMessages: (conversationId: string) =>
    invoke<MessageSummary[]>("conversation_messages", { conversationId }),
  chatSend: (
    conversationId: string,
    text: string,
    routing: RoutingMode,
    model: string,
    confirmed: boolean,
  ) =>
    invoke<ChatSendResult>("chat_send", {
      conversationId,
      text,
      routing,
      model,
      confirmed,
    }),

  // Identity
  identityGet: () => invoke<CompanionIdentity>("identity_get"),
  identitySave: (identity: CompanionIdentity, editSummary: string) =>
    invoke<void>("identity_save", { identity, editSummary }),

  // Knowledge
  knowledgeList: () => invoke<KnowledgeRecordView[]>("knowledge_list"),
  knowledgeAddFact: (text: string) => invoke<string>("knowledge_add_fact", { text }),
  knowledgeApprove: (recordId: string) => invoke<void>("knowledge_approve", { recordId }),
  knowledgeReject: (recordId: string) => invoke<void>("knowledge_reject", { recordId }),
  knowledgeTombstone: (recordId: string) =>
    invoke<void>("knowledge_tombstone", { recordId }),
  knowledgeCorrect: (recordId: string, newText: string) =>
    invoke<string>("knowledge_correct", { recordId, newText }),
  knowledgeSearch: (query: string, limit?: number) =>
    invoke<string[]>("knowledge_search", { query, limit: limit ?? 20 }),

  // Sources
  sourceImport: (filePath: string) => invoke<string>("source_import", { filePath }),
  sourceList: () => invoke<SourceView[]>("source_list"),
  sourceExtracted: (sourceId: string) => invoke<string>("source_extracted", { sourceId }),
  sourceDelete: (sourceId: string) => invoke<void>("source_delete", { sourceId }),
  sourceSearch: (query: string, limit?: number) =>
    invoke<string[]>("source_search", { query, limit: limit ?? 20 }),

  // Audit
  auditList: (limit?: number) =>
    invoke<AuditEventView[]>("audit_list", { limit: limit ?? 100 }),
  auditCounts: () => invoke<[string, number][]>("audit_counts"),

  // Memory
  memoryList: (stateFilter?: string) =>
    invoke<MemoryCandidateView[]>("memory_list", { stateFilter: stateFilter ?? null }),
  memoryApprove: (candidateId: string, editedText?: string) =>
    invoke<string>("memory_approve", { candidateId, editedText: editedText ?? null }),
  memoryReject: (candidateId: string) => invoke<void>("memory_reject", { candidateId }),
  memoryDefer: (candidateId: string) => invoke<void>("memory_defer", { candidateId }),
  memoryMarkTemporary: (candidateId: string) =>
    invoke<void>("memory_mark_temporary", { candidateId }),
  memoryDelete: (candidateId: string) => invoke<void>("memory_delete", { candidateId }),

  // Permissions
  toolRegistry: () => invoke<ToolDeclarationView[]>("tool_registry"),
  permissionListActive: () => invoke<PermissionGrantView[]>("permission_list_active"),
  permissionGrant: (toolId: string, mode: string, scope: string, expiresAt?: string) =>
    invoke<string>("permission_grant", {
      toolId,
      mode,
      scope,
      expiresAt: expiresAt ?? null,
    }),
  permissionRevoke: (toolId: string) => invoke<void>("permission_revoke", { toolId }),

  // Provider config (KoboldCpp connectivity)
  providerConfigGet: () => invoke<ProviderConfig>("provider_config_get"),
  providerConfigSave: (config: ProviderConfig) =>
    invoke<void>("provider_config_save", { config }),
  veniceApiKeySet: (apiKey: string) => invoke<void>("venice_api_key_set", { apiKey }),
  veniceTestConnection: () => invoke<string[]>("venice_test_connection"),
  koboldcppTestConnection: (endpoint: string) =>
    invoke<string[]>("koboldcpp_test_connection", { endpoint }),
  koboldcppChat: (
    endpoint: string,
    model: string,
    systemPrompt: string,
    userMessage: string,
    maxTokens?: number,
  ) =>
    invoke<{
      content: string;
      provider: string;
      model: string;
      crossed_to_cloud: boolean;
    }>("koboldcpp_chat", {
      endpoint,
      model,
      systemPrompt,
      userMessage,
      maxTokens: maxTokens ?? 256,
    }),
};

export type LockPolicy =
  "minutes_5" | "minutes_15" | "minutes_30" | "minutes_60" | "manual_only";

export type RoutingMode =
  "local_only" | "prefer_local" | "prefer_cloud" | "cloud_only" | "ask_before_crossing";

export const ROUTING_MODE_LABELS: Record<RoutingMode, string> = {
  local_only: "Local only",
  prefer_local: "Prefer local",
  prefer_cloud: "Prefer cloud",
  cloud_only: "Cloud only",
  ask_before_crossing: "Ask before crossing (default)",
};

export interface ConversationSummary {
  conversation_id: string;
  title: string | null;
  created_at: string;
  updated_at: string;
}

export interface MessageSummary {
  message_id: string;
  role: string;
  content: string;
  seq: number;
}

export interface CloudConsentView {
  provider_id: string;
  provider_display_name: string;
  model: string;
  routing_mode: string;
}

export interface PromptSectionSummary {
  id: string;
  title: string;
  body_chars: number;
}

export interface ChatSendResult {
  content: string | null;
  provider: string | null;
  model: string | null;
  crossed_to_cloud: boolean;
  consent_required: CloudConsentView | null;
  prompt_summary: PromptSectionSummary[];
}

export interface IdentityVersionEntry {
  version: number;
  changed_at: string;
  summary: string;
}

export interface CompanionIdentity {
  version: number;
  companion_name: string;
  role: string;
  core_principles: string[];
  conversational_traits: string[];
  tone_preferences: string[];
  user_communication_preferences: string[];
  boundaries: string[];
  honesty_requirements: string[];
  uncertainty_behavior: string;
  privacy_rules: string[];
  memory_rules: string[];
  tool_use_rules: string[];
  approved_knowledge_references: string[];
  version_history: IdentityVersionEntry[];
}

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

export interface KnowledgeRecordView {
  record_id: string;
  record_type: string;
  canonical_text: string;
  status: string;
  review_state: string;
  sensitivity: string;
  confidence: number;
  domain_tags: string[];
  source_ids: string[];
  supersedes: string | null;
  superseded_by: string | null;
  contradiction_set: string | null;
  created_at: string;
  updated_at: string;
}

export interface SourceView {
  source_id: string;
  kind: string;
  name: string;
  checksum: string;
  imported_at: string;
  status: string;
  bytes_len: number;
}

export interface AuditEventView {
  seq: number;
  event_id: string;
  occurred_at: string;
  actor: string | null;
  category: string;
  action: string;
  detail_json: unknown;
}

export interface MemoryCandidateView {
  candidate_id: string;
  proposed_text: string;
  record_type: string;
  source_conversation_id: string | null;
  source_message_id: string | null;
  reason: string;
  confidence: number;
  sensitivity: string;
  suggested_domain: string;
  provider_used: string;
  crossed_to_cloud: boolean;
  created_at: string;
  state: string;
}

export interface ToolDeclarationView {
  tool_id: string;
  operation: string;
  data_accessed: string;
  data_leaving_device: string;
  destination: string;
  risk: string;
  reversibility: string;
}

export interface PermissionGrantView {
  grant_id: string;
  tool_id: string;
  mode: string;
  scope: string;
  granted_at: string;
  expires_at: string | null;
}

export interface ProviderConfig {
  koboldcpp_endpoint: string;
  koboldcpp_model: string;
  koboldcpp_enabled: boolean;
  venice_endpoint: string;
  venice_model: string;
  venice_enabled: boolean;
  venice_has_api_key: boolean;
}
