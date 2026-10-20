// What Baylee does, nine things in a three-by-three grid, in both languages.

import type { IconName } from "../components/icons";

export const FEATURES: { icon: IconName; title: [string, string]; text: [string, string] }[] = [
  {
    icon: "eye",
    title: ["Hidden information stays hidden", "Verdeckte Information bleibt verdeckt"],
    text: [
      "Your client is sent only what you may see. Libraries and other hands arrive as counts, a face-down card carries no identity. A modified client cannot peek, because the data never reaches it.",
      "Dein Client bekommt nur, was du sehen darfst. Bibliotheken und fremde Hände kommen als Zahlen an, eine verdeckte Karte trägt keine Identität. Ein manipulierter Client kann nicht spicken, weil die Daten ihn nie erreichen.",
    ],
  },
  {
    icon: "people",
    title: ["Two to eight seats", "Zwei bis acht Plätze"],
    text: [
      "Duels, free-for-all and team tables. The host puts chairs on sides, and the sides need not be even. Teams keep separate turns and life totals; teammates can choose to show each other their hands.",
      "Duelle, Jeder-gegen-jeden und Teamtische. Der Gastgeber verteilt die Stühle auf Seiten, und die Seiten müssen nicht gleich groß sein. Teams haben eigene Züge und Lebenspunkte; Teamkollegen können sich gegenseitig die Hand zeigen.",
    ],
  },
  {
    icon: "crown",
    title: ["Commander and freeform", "Commander und Freiform"],
    text: [
      "A deck that names a commander plays Commander; any other deck plays freeform. Banlists and format-legality checks do not exist yet. That is a gap, not a feature.",
      "Ein Deck mit Commander spielt Commander, jedes andere spielt frei. Banlisten und Formatprüfungen gibt es noch nicht. Das ist eine Lücke, kein Feature.",
    ],
  },
  {
    icon: "chip",
    title: ["A house AI at five levels", "Eine Haus-KI in fünf Stufen"],
    text: [
      "Novice, casual, steady, sharp and expert. It plays from the same view a human gets, so it cannot see your hand either.",
      "Novice, casual, steady, sharp und expert. Sie spielt aus derselben Sicht wie ein Mensch und sieht deine Hand genauso wenig.",
    ],
  },
  {
    icon: "grid",
    title: ["One process per game", "Ein Prozess pro Spiel"],
    text: [
      "Each game runs in its own engine process. The gateway that handles accounts and rooms runs no rules at all, so one broken game is one broken game.",
      "Jedes Spiel läuft in einem eigenen Engine-Prozess. Das Gateway für Konten und Räume kennt keine Regeln, also ist ein kaputtes Spiel genau ein kaputtes Spiel.",
    ],
  },
  {
    icon: "globe",
    title: ["Card text in 19 languages", "Kartentext in 19 Sprachen"],
    text: [
      "When the gateway carries the card catalog, cards read in your language with English as the fallback. The interface itself is in English and German.",
      "Hat das Gateway den Kartenkatalog, lesen sich Karten in deiner Sprache, Englisch als Rückfall. Die Oberfläche selbst gibt es auf Englisch und Deutsch.",
    ],
  },
  {
    icon: "watch",
    title: ["Watch a running table", "Einem laufenden Tisch zuschauen"],
    text: [
      "The lobby's Watch button seats you as a spectator. You see what the whole table sees and nobody's hand. A host can switch spectators off for a room.",
      "Der Knopf Zuschauen in der Lobby setzt dich als Zuschauer an den Tisch. Du siehst, was der ganze Tisch sieht, und keine Hand. Ein Gastgeber kann Zuschauer für seinen Raum abschalten.",
    ],
  },
  {
    icon: "wifi",
    title: ["A lost connection is not a lost game", "Verbindung weg, Spiel nicht verloren"],
    text: [
      "When a player drops, the table shows a countdown while their chair waits for them. If time runs out, the house AI plays for them until they are back; with nobody else at the table, the game simply pauses.",
      "Fällt ein Spieler heraus, zeigt der Tisch einen Countdown, solange sein Platz auf ihn wartet. Läuft die Zeit ab, spielt die Haus-KI für ihn, bis er zurück ist; sitzt sonst niemand am Tisch, pausiert das Spiel einfach.",
    ],
  },
  {
    icon: "sliders",
    title: ["Tables your way", "Tische nach deinen Regeln"],
    text: [
      "A Mixed table seats decks of any format together, and fourteen decision clocks run from a 15-second bullet to untimed. Starting life and mulligans are the host's to set.",
      "Ein Tisch „Gemischt“ setzt Decks jedes Formats zusammen, und vierzehn Entscheidungsuhren reichen von 15 Sekunden bis ohne Uhr. Startleben und Mulligans legt der Gastgeber fest.",
    ],
  },
];
