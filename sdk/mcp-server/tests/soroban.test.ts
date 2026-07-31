import { describe, expect, it } from "vitest";
import { describeContractError } from "../src/soroban.js";

describe("describeContractError", () => {
  it("translates a known contract error code", () => {
    const raw =
      "HostError: Error(Contract, #13)\n\nEvent log (newest first):\n   0: [Diagnostic Event] ...";
    expect(describeContractError(raw)).toBe("Contract error 13: No snapshot found for the requested epoch");
  });

  it("falls back gracefully for an unrecognized code", () => {
    expect(describeContractError("Error(Contract, #999)")).toBe("Contract error 999 (unrecognized)");
  });

  it("returns the raw message unchanged when it isn't a contract error", () => {
    const raw = "network timeout";
    expect(describeContractError(raw)).toBe(raw);
  });
});
