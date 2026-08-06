import { describe, expect, it } from "vitest";
import { explainError } from "./tauri";

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
