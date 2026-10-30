import { describe, expect, it } from "vitest";

import { gatewayLink } from "../../src/central/deeplink";

describe("a gateway's deep link", () => {
  it("carries the origin and nothing else", () => {
    expect(gatewayLink("https://gateway.invalid")).toBe("baylee://gateway/add?origin=https%3A%2F%2Fgateway.invalid");
    expect(gatewayLink("https://gateway.invalid:8443")).toBe("baylee://gateway/add?origin=https%3A%2F%2Fgateway.invalid%3A8443");
  });

  it("refuses anything that is not a bare https origin", () => {
    for (const origin of [
      "http://gateway.invalid",
      "https://gateway.invalid/",
      "https://gateway.invalid/path",
      "https://u:p@gateway.invalid",
      "https://GATEWAY.invalid",
      "https://gateway.invalid?x=1",
      "javascript:alert(1)",
      "baylee://gateway/add?origin=https://x.invalid",
      "",
      `https://${"a".repeat(260)}.invalid`,
      // hosts the client ignores: an IP address or a single label
      "https://203.0.113.7",
      "https://[2001:db8::1]",
      "https://localhost",
      "https://printer:8443",
    ]) {
      expect({ origin, link: gatewayLink(origin) }).toEqual({ origin, link: null });
    }
  });

  it("takes a loopback http origin only on a page served from this machine", () => {
    expect(gatewayLink("http://127.0.0.1:28790")).toBeNull();
    expect(gatewayLink("http://127.0.0.1:28790", true)).toBe("baylee://gateway/add?origin=http%3A%2F%2F127.0.0.1%3A28790");
    expect(gatewayLink("http://192.168.0.2:28790", true)).toBeNull();
  });
});
