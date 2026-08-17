import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockRejectedValue(new Error("no backend")),
}));

import { VaultView } from "./VaultView";
import { invoke } from "@tauri-apps/api/core";

describe("VaultView passphrase fields", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("unlocks using the live field value when React state was not updated", async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "vault_list") {
        return [
          {
            vault_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            name: "pkc-keep-smoke",
            template: "personal",
            created_at: "2026-08-17T00:00:00Z",
          },
        ];
      }
      if (command === "vault_status") {
        return { unlocked: false, active_vault_id: null };
      }
      if (command === "lock_policy_get") {
        return "minutes_15";
      }
      if (command === "vault_unlock") {
        return {
          vault_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
          name: "pkc-keep-smoke",
          template: "personal",
          created_at: "2026-08-17T00:00:00Z",
        };
      }
      throw new Error(`unexpected command: ${command}`);
    });

    render(<VaultView />);
    const unlock = await screen.findByRole("button", { name: "Unlock" });
    const field = screen.getByLabelText("Vault passphrase") as HTMLInputElement;
    field.value = "smoketest-pkc-ui";
    fireEvent.click(unlock);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("vault_unlock", {
        vaultId: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        passphrase: "smoketest-pkc-ui",
      });
    });
  });

  it("rejects an empty unlock field without invoking authentication", async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "vault_list") {
        return [
          {
            vault_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            name: "pkc-keep-smoke",
            template: "personal",
            created_at: "2026-08-17T00:00:00Z",
          },
        ];
      }
      if (command === "vault_status") {
        return { unlocked: false, active_vault_id: null };
      }
      if (command === "lock_policy_get") {
        return "minutes_15";
      }
      throw new Error(`unexpected command: ${command}`);
    });

    render(<VaultView />);
    fireEvent.click(await screen.findByRole("button", { name: "Unlock" }));

    expect(await screen.findByText(/Passphrase is required/)).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith("vault_unlock", expect.anything());
  });

  it("creates a vault using the live master-passphrase field", async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "vault_list") return [];
      if (command === "vault_status") {
        return { unlocked: false, active_vault_id: null };
      }
      if (command === "lock_policy_get") return "minutes_15";
      if (command === "vault_create") {
        return {
          vault: {
            vault_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            name: "pkc-keep-smoke",
            template: "personal",
            created_at: "2026-08-17T00:00:00Z",
          },
          recovery_code: "AAAA-BBBB-CCCC-DDDD",
        };
      }
      throw new Error(`unexpected command: ${command}`);
    });

    render(<VaultView />);
    await screen.findByRole("button", { name: "Create vault" });
    const name = screen.getByLabelText("Name") as HTMLInputElement;
    const pass = screen.getByLabelText(
      "Master passphrase (min 8 characters)",
    ) as HTMLInputElement;
    name.value = "pkc-keep-smoke";
    pass.value = "smoketest-pkc-ui";
    fireEvent.click(screen.getByRole("button", { name: "Create vault" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("vault_create", {
        name: "pkc-keep-smoke",
        template: "personal",
        passphrase: "smoketest-pkc-ui",
      });
    });
  });
});
