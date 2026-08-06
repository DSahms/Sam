import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Sidebar } from "./Sidebar";
import { views } from "./navigation";

describe("Sidebar", () => {
  it("renders all eight primary navigation destinations", () => {
    render(<Sidebar active="vaults" onSelect={() => {}} />);
    for (const label of Object.values(views).map((v) => v.label)) {
      expect(screen.getByRole("button", { name: label })).toBeInTheDocument();
    }
  });

  it("marks the active destination", () => {
    render(<Sidebar active="chat" onSelect={() => {}} />);
    const chat = screen.getByRole("button", { name: "Chat" });
    expect(chat).toHaveAttribute("aria-current", "page");
  });

  it("notifies on selection", () => {
    const onSelect = vi.fn();
    render(<Sidebar active="vaults" onSelect={onSelect} />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(onSelect).toHaveBeenCalledWith("settings");
  });
});
