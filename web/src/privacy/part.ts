import type { Key } from "../i18n";

/** One section of the notice: its title, its paragraphs and its legal basis, as messages. */
export interface Part {
  id: string;
  title: Key;
  body: Key[];
  basis?: Key;
}
