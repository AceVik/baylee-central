// The strength hint under the password field: a hint, never a rule (the
// design asks for no composition rules). It estimates the guessing effort
// from the length and the kinds of characters, and lowers it for what
// guessers try first: one character repeated, runs like `abc` or `321`, the
// name, and a few words everyone uses. Eight characters are the minimum;
// fifteen or more make a strong passphrase on their own (NIST SP 800-63B).

export type Strength = "short" | "weak" | "fair" | "strong";

export const MIN_LENGTH = 8;
/** The crypto's bound: 1024 bytes of UTF-8. */
export const MAX_BYTES = 1024;

const COMMON = [
  "password",
  "passwort",
  "qwerty",
  "qwertz",
  "letmein",
  "welcome",
  "willkommen",
  "baylee",
  "magic",
  "gathering",
  "admin",
  "hallo",
  "hello",
  "dragon",
  "monkey",
  "123456",
  "654321",
  "abc123",
  "iloveyou",
];

function pool(chars: readonly string[]): number {
  let size = 0;
  if (chars.some((c) => /[a-z]/.test(c))) size += 26;
  if (chars.some((c) => /[A-Z]/.test(c))) size += 26;
  if (chars.some((c) => /[0-9]/.test(c))) size += 10;
  if (chars.some((c) => /[\x20-\x2f\x3a-\x40\x5b-\x60\x7b-\x7e]/.test(c))) size += 33;
  if (chars.some((c) => c > "\x7f")) size += 100;
  return Math.max(size, 1);
}

/** How many characters continue a run (`abc`, `cba`, `111`) from the one before. */
function runs(chars: readonly string[]): number {
  let n = 0;
  for (let i = 1; i < chars.length; i++) {
    const d = (chars[i]?.codePointAt(0) ?? 0) - (chars[i - 1]?.codePointAt(0) ?? 0);
    if (Math.abs(d) <= 1) n++;
  }
  return n;
}

export function strength(password: string, username = ""): Strength {
  const chars = Array.from(password);
  if (chars.length < MIN_LENGTH) return "short";
  const lower = password.toLowerCase();
  const name = username.trim().toLowerCase();
  const guessable =
    new Set(chars).size <= 2 || COMMON.some((w) => lower.includes(w)) || (name.length >= 3 && lower.includes(name));
  // each character in a run, or repeated, adds little
  const effective = Math.max(1, chars.length - runs(chars) * 0.75);
  const bits = effective * Math.log2(pool(chars)) * (guessable ? 0.5 : 1);
  if (bits < 40) return "weak";
  if (bits < 65 && chars.length < 15) return "fair";
  return guessable ? "fair" : "strong";
}

/** Whether `password` may be sent at all: at least eight characters, at most 1024 bytes. */
export function acceptable(password: string): boolean {
  return Array.from(password).length >= MIN_LENGTH && new TextEncoder().encode(password).length <= MAX_BYTES;
}
