// A reporter's pseudonym as something an administrator tells apart at a
// glance (Baylee `web/feedback/src/reports/alias.ts`).
//
// The pseudonym is an HMAC the gateway makes under a key only it holds, or a
// device's HMAC (docs/reports.md §"Privacy invariants"): Central cannot read
// an account from it, and that is the point. This page never asks anyone to
// turn it back into a name. It derives two words from the hash itself, so
// the same reporter reads the same in every row and another one reads
// differently, and nothing more can be read from them than from the hex.

const ADJECTIVES = [
  "Amber", "Bold", "Brisk", "Calm", "Clever", "Cool", "Crisp", "Dapper",
  "Deep", "Eager", "Fair", "Fleet", "Fond", "Frank", "Gentle", "Glad",
  "Grand", "Hardy", "Hollow", "Humble", "Jolly", "Keen", "Kind", "Late",
  "Light", "Lively", "Loyal", "Lucky", "Merry", "Mild", "Misty", "Neat",
  "Noble", "Odd", "Pale", "Plain", "Proud", "Quick", "Quiet", "Rapid",
  "Rare", "Ready", "Rosy", "Royal", "Rusty", "Sharp", "Shy", "Silent",
  "Sly", "Soft", "Solid", "Spry", "Steady", "Still", "Stout", "Swift",
  "Tidy", "True", "Vivid", "Warm", "Wild", "Wise", "Witty", "Young",
] as const;

const NOUNS = [
  "Badger", "Bear", "Beetle", "Bison", "Crane", "Crow", "Deer", "Dove",
  "Eagle", "Eel", "Elk", "Falcon", "Ferret", "Finch", "Fox", "Frog",
  "Gull", "Hare", "Hawk", "Heron", "Hornet", "Ibis", "Jay", "Koi",
  "Lark", "Lemur", "Lion", "Llama", "Lynx", "Mole", "Moose", "Moth",
  "Newt", "Orca", "Otter", "Owl", "Panda", "Pike", "Puma", "Quail",
  "Raven", "Robin", "Seal", "Shrew", "Snail", "Sparrow", "Stag", "Stork",
  "Swan", "Swift", "Tapir", "Tern", "Toad", "Trout", "Viper", "Vole",
  "Wasp", "Whale", "Wolf", "Wren", "Yak", "Zebra", "Carp", "Crab",
] as const;

export interface Alias {
  /** Two words, such as `Quiet Heron`. */
  name: string;
  /** The first four hex digits, which tell two reporters of one name apart. */
  short: string;
}

/** The alias of `pseudonym` (hex), the same every time. */
export function alias(pseudonym: string): Alias {
  const n = Number.parseInt(pseudonym.slice(0, 8).padEnd(8, "0"), 16) >>> 0;
  return {
    name: `${ADJECTIVES[n % ADJECTIVES.length] ?? "Plain"} ${NOUNS[Math.floor(n / 64) % NOUNS.length] ?? "Wren"}`,
    short: pseudonym.slice(0, 4).toLowerCase(),
  };
}
