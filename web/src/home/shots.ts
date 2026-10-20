// The client's screenshots: file names under MEDIA_BASE (`npm run content`),
// alt texts and captions in both languages.

export interface Shot {
  file: string;
  alt: [string, string];
  title: [string, string];
  caption: [string, string];
}

export const SHOTS: Shot[] = [
  {
    file: "shot-table-duel",
    alt: ["A duel against the house AI; a hovered card is shown large in the text view", "Ein Duell gegen die Haus-KI; eine Karte unter dem Zeiger groß in der Textansicht"],
    title: ["A duel", "Ein Duell"],
    caption: ["against the house AI. Hovering a card shows it large.", "gegen die Haus-KI. Eine Karte unter dem Zeiger wird groß gezeigt."],
  },
  {
    file: "shot-game-log",
    alt: ["The game log beside the table, each card name a link", "Das Spielprotokoll neben dem Tisch, jeder Kartenname ein Link"],
    title: ["The game log", "Das Spielprotokoll"],
    caption: ["(L) reads like a chat; every card name opens its preview.", "(L) liest sich wie ein Chat; jeder Kartenname öffnet die Vorschau."],
  },
  {
    file: "shot-table-ring",
    alt: ["A four-seat table, the seats in a ring around the felt", "Ein Tisch mit vier Plätzen im Ring um den Filz"],
    title: ["Four seats.", "Vier Plätze."],
    caption: ["Larger tables sit in a ring around the felt.", "Größere Tische sitzen im Ring um den Filz."],
  },
  {
    file: "shot-lobby",
    alt: ["The lobby: decks on the left, open tables on the right", "Die Lobby: Decks links, offene Tische rechts"],
    title: ["The lobby.", "Die Lobby."],
    caption: ["Your decks, the open tables, and Play the house for a quick game.", "Deine Decks, die offenen Tische und Play the house für ein schnelles Spiel."],
  },
  {
    file: "shot-room",
    alt: ["A room being arranged: seats, AI levels, starting life", "Ein Raum wird eingerichtet: Plätze, KI-Stufen, Startleben"],
    title: ["Arranging a table.", "Einen Tisch einrichten."],
    caption: ["Chairs, starting life, mulligans, and a level for each AI chair.", "Stühle, Startleben, Mulligans und eine Stufe für jeden KI-Stuhl."],
  },
  {
    file: "shot-deck-builder",
    alt: ["The deck builder with statistics and a search over the pool", "Der Deckbauer mit Statistik und einer Suche über den Pool"],
    title: ["The deck builder.", "Der Deckbauer."],
    caption: [
      "Mana curve, land statistics, a search over the whole pool, and a warning for cards that will not play as printed.",
      "Manakurve, Länderstatistik, eine Suche über den ganzen Pool und eine Warnung für Karten, die nicht wie gedruckt spielen.",
    ],
  },
];
