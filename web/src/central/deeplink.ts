// The link that opens Baylee's client on one gateway, in the grammar of
// Baylee's deep-link design (`docs/proposals/deep-links.md` on
// `c41/deep-links-design`): `baylee://gateway/add?origin=<origin>`. The link
// carries one public origin and nothing else; the client shows the gateway
// and asks before it saves it. A client that does not know the scheme yet
// leaves the link unanswered, so the page always offers the address too.

/** The scheme the installers register (deep-link design, question 6). */
export const SCHEME = "baylee";

/**
 * The deep link for a gateway at `origin`, or null for anything that is not
 * a bare https origin (or, on a page served from this machine, a loopback
 * http one): the link never carries a path, a query, credentials or text.
 */
export function gatewayLink(origin: string, loopback = false): string | null {
  let url: URL;
  try {
    url = new URL(origin);
  } catch {
    return null;
  }
  const local = loopback && url.protocol === "http:" && /^(127\.0\.0\.1|\[::1\])$/.test(url.hostname);
  if ((url.protocol !== "https:" && !local) || url.username !== "" || url.password !== "" || url.origin !== origin || origin.length > 256) {
    return null;
  }
  return `${SCHEME}://gateway/add?origin=${encodeURIComponent(origin)}`;
}
