// Central's name rules; Central decides, the page only says early what it
// would refuse.
//
// The username (central-postgres's canonical name, the accounts
// table's CHECK): 3 to 24 characters, letters a–z, digits and `_ . -`,
// starting and ending with a letter or digit, never two of `_ . -` in a row.
// Capitals are read as small letters. Central decides; the page only says
// early what it would refuse.

export function canonicalName(value: string): string {
  return value.trim().toLowerCase();
}

export function validName(value: string): boolean {
  const name = canonicalName(value);
  if (name.length < 3 || name.length > 24 || !/^[a-z0-9_.-]+$/.test(name)) return false;
  if (!/^[a-z0-9]/.test(name) || !/[a-z0-9]$/.test(name)) return false;
  return !/[_.-]{2}/.test(name);
}

// The display name other players see (central-api's `profile`, the
// gateway's rule exactly): 3 to 16 ASCII letters and digits, with `_` and
// `-` only between them; case is kept. Names are not unique: the tag beside
// it tells two apart. A person may not choose one of Central's reserved
// names, read without case, `_` and `-`.

const DISPLAY_NAME = /^[A-Za-z0-9][A-Za-z0-9_-]{1,14}[A-Za-z0-9]$/;
const RESERVED = new Set(["admin", "administrator", "baylee", "central", "moderator", "official", "support", "system"]);
/** A tag is positive and fits a gateway's column (`i32`). */
export const MAX_TAG = 2 ** 31 - 1;

export function validDisplayName(value: string): boolean {
  return DISPLAY_NAME.test(value);
}

export function reservedDisplayName(value: string): boolean {
  return RESERVED.has(value.replace(/[_-]/g, "").toLowerCase());
}

export function validTag(tag: number): boolean {
  return Number.isInteger(tag) && tag >= 1 && tag <= MAX_TAG;
}

/** A tag as a player reads it: lowercase hex, at least four digits (`7` is `0007`). */
export function tagText(tag: number): string {
  return tag.toString(16).padStart(4, "0");
}

/** The whole handle, `Alice#0007`. */
export function handle(name: string, tag: number): string {
  return `${name}#${tagText(tag)}`;
}
