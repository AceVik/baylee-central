// What Baylee does, nine things in a three-by-three grid: an icon and two
// messages each.

import type { IconName } from "../components/icons";
import type { Key } from "../i18n";

export const FEATURES: { icon: IconName; title: Key; text: Key }[] = [
  { icon: "eye", title: "feature.hiddenTitle", text: "feature.hidden" },
  { icon: "people", title: "feature.seatsTitle", text: "feature.seats" },
  { icon: "crown", title: "feature.commanderTitle", text: "feature.commander" },
  { icon: "chip", title: "feature.aiTitle", text: "feature.ai" },
  { icon: "grid", title: "feature.processTitle", text: "feature.process" },
  { icon: "globe", title: "feature.languagesTitle", text: "feature.languages" },
  { icon: "watch", title: "feature.watchTitle", text: "feature.watch" },
  { icon: "wifi", title: "feature.connectionTitle", text: "feature.connection" },
  { icon: "sliders", title: "feature.tablesTitle", text: "feature.tables" },
];
