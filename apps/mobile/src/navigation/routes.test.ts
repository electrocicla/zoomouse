import { describe, expect, it } from "vitest";
import { navigationTitle } from "./routes";

describe("M1-B local navigation", () => {
  it("names distinct shell routes", () => {
    expect(navigationTitle("devices")).toBe("Dispositivos");
    expect(navigationTitle("manual-ip")).not.toBe(navigationTitle("qr"));
    expect(navigationTitle("controls")).toBe("Control remoto");
  });
});
