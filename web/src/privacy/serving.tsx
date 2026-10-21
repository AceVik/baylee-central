// The notice's first half: who is responsible, and what serving the site
// sends where (the server, the browser's own storage, GitHub, Scryfall,
// Central).

import type { Part } from "./part";

export const SERVING: Part[] = [
  { id: "controller", title: "privacy.controllerTitle", body: ["privacy.controller"] },
  { id: "hosting", title: "privacy.hostingTitle", body: ["privacy.hosting"], basis: "privacy.hostingBasis" },
  { id: "storage", title: "privacy.storageTitle", body: ["privacy.storage"], basis: "privacy.storageBasis" },
  { id: "github", title: "privacy.githubTitle", body: ["privacy.github"], basis: "privacy.githubBasis" },
  { id: "scryfall", title: "privacy.scryfallTitle", body: ["privacy.scryfall"], basis: "privacy.scryfallBasis" },
  { id: "central", title: "privacy.centralTitle", body: ["privacy.central"], basis: "privacy.centralBasis" },
];
