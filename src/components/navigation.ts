// Navigation metadata, separated from the Sidebar component so the component
// file only exports components (keeps react-refresh happy and lets views reuse
// the metadata without pulling in JSX).
export type ViewId =
  | "vaults"
  | "chat"
  | "whatiknow"
  | "sources"
  | "memory"
  | "privacy"
  | "backup"
  | "settings";

export const views: Record<ViewId, { label: string; phase: string }> = {
  vaults: { label: "Vaults", phase: "Phase 1" },
  chat: { label: "Chat", phase: "Phase 2" },
  whatiknow: { label: "What I Know", phase: "Phase 3" },
  sources: { label: "Sources", phase: "Phase 4" },
  memory: { label: "Memory Review", phase: "Phase 7" },
  privacy: { label: "Privacy & Audit", phase: "Phase 1+" },
  backup: { label: "Backup & Recovery", phase: "Phase 1 / 9" },
  settings: { label: "Settings", phase: "Phase 1" },
};

export const navOrder: ViewId[] = [
  "vaults",
  "chat",
  "whatiknow",
  "sources",
  "memory",
  "privacy",
  "backup",
  "settings",
];
