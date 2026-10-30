/** Validate only RFC1918 IPv4 literals. Does not open sockets. */
export const parsePrivateIpv4 = (input: string): string | null => {
  const parts = input.trim().split(".");
  if (parts.length !== 4) return null;
  const octets = parts.map(part => {
    if (!/^(0|[1-9][0-9]{0,2})$/.test(part)) return -1;
    const value = Number(part);
    return value <= 255 ? value : -1;
  });
  const [first, second, , last] = octets;
  // Keep the mobile address gate consistent with the published pairing-wire host endpoint.
  if (first === undefined || second === undefined || last === undefined ||
      last === 0 || last === 255 || octets.some(value => value < 0)) return null;
  const privateLan = first === 10 || (first === 172 && second >= 16 && second <= 31) || (first === 192 && second === 168);
  return privateLan ? octets.join(".") : null;
};
