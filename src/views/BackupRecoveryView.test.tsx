import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

// Mock the Tauri APIs before importing the component.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockRejectedValue(new Error("no backend")),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue(null),
  save: vi.fn().mockResolvedValue(null),
}));

import { BackupRecoveryView } from "./BackupRecoveryView";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

describe("BackupRecoveryView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  async function renderSettled() {
    const result = render(<BackupRecoveryView />);
    await screen.findByText(/no backend/);
    return result;
  }

  it("renders the Create Backup section", async () => {
    await renderSettled();
    expect(screen.getByRole("heading", { name: "Create Backup" })).toBeInTheDocument();
  });

  it("renders the Restore from Backup section", async () => {
    await renderSettled();
    expect(screen.getByText("Restore from Backup")).toBeInTheDocument();
  });

  it("shows the backup destination chooser button", async () => {
    await renderSettled();
    expect(screen.getByText("Choose destination…")).toBeInTheDocument();
  });

  it("shows the restore file selector button", async () => {
    await renderSettled();
    expect(screen.getByText("Select backup file…")).toBeInTheDocument();
  });

  it("renders without crashing when backend is unavailable", async () => {
    // The component calls vaultStatus on mount which will fail; it should
    // render gracefully without throwing.
    const { container } = await renderSettled();
    expect(container.querySelector(".backup-view")).toBeTruthy();
  });

  it("requires confirmation and can restore with a recovery code", async () => {
    vi.mocked(open).mockResolvedValueOnce("C:\\backup.sammy-backup");
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "vault_status") {
        return { unlocked: false, active_vault_id: null };
      }
      if (command === "vault_backup_preview") {
        return {
          format_version: 1,
          vault_id: "vault-1234567890",
          vault_name: "Recovered vault",
          created_at: "2026-08-12T10:00:00Z",
        };
      }
      if (command === "vault_restore_recovery") return "vault-1234567890";
      throw new Error(`unexpected command: ${command}`);
    });
    render(<BackupRecoveryView />);
    fireEvent.click(screen.getByText("Select backup file…"));
    await screen.findByText("Recovered vault");
    expect(screen.queryByText("Recovery code for this backup")).toBeNull();
    fireEvent.click(screen.getByText("I understand — continue"));
    fireEvent.click(screen.getByRole("radio", { name: "Recovery code" }));
    fireEvent.change(screen.getByLabelText("Recovery code for this backup"), {
      target: { value: "ABCD-EFGH-IJKL" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Restore Backup" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("vault_restore_recovery", {
        inPath: "C:\\backup.sammy-backup",
        recoveryCode: "ABCD-EFGH-IJKL",
        confirmed: true,
      }),
    );
  });
});
