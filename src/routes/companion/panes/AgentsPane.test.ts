import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("companion Agents pane", () => {
  it("uses only the layered verification view", () => {
    const source = readFileSync(new URL("./AgentsPane.svelte", import.meta.url), "utf8");

    expect(source).toContain("WizardVerificationPanel");
    expect(source).not.toContain("MCPClientsPanel");
  });
});
