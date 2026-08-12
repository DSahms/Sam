import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";

// Mock the Tauri APIs before importing the component.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockRejectedValue(new Error("no backend")),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue(null),
  save: vi.fn().mockResolvedValue(null),
}));

import { BackupRecoveryView } from "./BackupRecoveryView";

describe("BackupRecoveryView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the Create Backup section", () => {
    render(<BackupRecoveryView />);
    expect(screen.getByRole("heading", { name: "Create Backup" })).toBeInTheDocument();
  });

  it("renders the Restore from Backup section", () => {
    render(<BackupRecoveryView />);
    expect(screen.getByText("Restore from Backup")).toBeInTheDocument();
  });

  it("shows the backup destination chooser button", () => {
    render(<BackupRecoveryView />);
    expect(screen.getByText("Choose destination…")).toBeInTheDocument();
  });

  it("shows the restore file selector button", () => {
    render(<BackupRecoveryView />);
    expect(screen.getByText("Select backup file…")).toBeInTheDocument();
  });

  it("renders without crashing when backend is unavailable", () => {
    // The component calls vaultStatus on mount which will fail; it should
    // render gracefully without throwing.
    const { container } = render(<BackupRecoveryView />);
    expect(container.querySelector(".backup-view")).toBeTruthy();
  });
});
