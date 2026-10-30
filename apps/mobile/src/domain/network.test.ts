import { describe, expect, it } from "vitest";
import { parsePrivateIpv4 } from "./network";

describe("manual LAN address validation", () => {
  it.each(["192.168.1.50", "10.0.0.12", "172.16.1.2", "172.31.255.254"])("accepts RFC1918 IPv4 %s", value => {
    expect(parsePrivateIpv4(value)).toBe(value);
  });
  it.each(["8.8.8.8", "172.32.0.1", "127.0.0.1", "169.254.1.1", "192.168.001.4", "192.168.1.256", "example.com", "", "::1", "192.168.1.2:9876", "10.0.0.0", "10.0.0.255", "192.168.1.0", "192.168.1.255"])("rejects non-private/malformed %s", value => {
    expect(parsePrivateIpv4(value)).toBeNull();
  });
});
