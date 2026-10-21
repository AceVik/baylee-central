// Central's username rule (central-postgres's canonical name, the accounts
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
