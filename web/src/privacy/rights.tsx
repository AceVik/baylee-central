// The notice's second half: the contact form, the legal notice's
// protection, links to other sites, and the visitor's rights.

import type { Part } from "./part";

export const PERSONAL: Part[] = [
  { id: "contact", title: "privacy.contactTitle", body: ["privacy.contact"], basis: "privacy.contactBasis" },
  { id: "impressum", title: "privacy.impressumTitle", body: ["privacy.impressum"], basis: "privacy.impressumBasis" },
  { id: "links", title: "privacy.linksTitle", body: ["privacy.links"] },
  { id: "rights", title: "privacy.rightsTitle", body: ["privacy.rights", "privacy.complain", "privacy.automated"] },
];
