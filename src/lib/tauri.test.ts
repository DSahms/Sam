import { describe, expect, it } from "vitest";
import {
  explainError,
  normalizeEpistemicClass,
  pkcClassLabel,
  suggestKeepText,
} from "./tauri";

describe("explainError", () => {
  it("explains an AppError-shaped object", () => {
    expect(
      explainError({ kind: "crypto", message: "cryptographic verification failed" }),
    ).toBe("cryptographic verification failed");
  });

  it("explains a generic Error", () => {
    expect(explainError(new Error("boom"))).toBe("boom");
  });

  it("explains an arbitrary value", () => {
    expect(explainError("nope")).toBe("nope");
  });
});

describe("PKC keep helpers", () => {
  it("labels stored fact, testimony, and inference in plain language", () => {
    expect(pkcClassLabel("stored_fact")).toBe("Stored personal knowledge");
    expect(pkcClassLabel("testimony")).toBe("Something you described");
    expect(pkcClassLabel("inference")).toBe("A suggestion to review");
  });

  it("defaults unknown classes to stored personal knowledge", () => {
    expect(normalizeEpistemicClass("mystery")).toBe("stored_fact");
    expect(normalizeEpistemicClass("testimony")).toBe("testimony");
  });

  it("prefills a short first sentence, not the whole answer", () => {
    expect(suggestKeepText("You grew up in Gloucester Township. The rest is extra.")).toBe(
      "You grew up in Gloucester Township.",
    );
  });
});
