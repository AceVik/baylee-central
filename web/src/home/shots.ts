// The client's screenshots: file names under MEDIA_BASE (`npm run content`),
// and the messages for each one's alt text, title and caption.

import type { Key } from "../i18n";

export interface Shot {
  file: string;
  alt: Key;
  title: Key;
  caption: Key;
}

export const SHOTS: Shot[] = [
  { file: "shot-table-duel", alt: "shot.duelAlt", title: "shot.duelTitle", caption: "shot.duelCaption" },
  { file: "shot-game-log", alt: "shot.logAlt", title: "shot.logTitle", caption: "shot.logCaption" },
  { file: "shot-table-ring", alt: "shot.ringAlt", title: "shot.ringTitle", caption: "shot.ringCaption" },
  { file: "shot-lobby", alt: "shot.lobbyAlt", title: "shot.lobbyTitle", caption: "shot.lobbyCaption" },
  { file: "shot-room", alt: "shot.roomAlt", title: "shot.roomTitle", caption: "shot.roomCaption" },
  { file: "shot-deck-builder", alt: "shot.builderAlt", title: "shot.builderTitle", caption: "shot.builderCaption" },
];
