// The start page:
// 1. StatusStrip (Repo Stats Bar) & Hero
// 2. Am Tisch (#screens)
// 3. Registrierte Gateways (#gateways)
// 4. Was es ist & KI-Integration (#about / #ai)
// 5. Kontakt (#contact)
// 6. Quelloffen (#source)

import { CI_PATH, COMMIT_PATH, parseCi, parseCommit, useGitHub } from "../github";
import { useTitle } from "../i18n";
import "../styles/home.css";
import { About } from "./About";
import { Hero } from "./Hero";
import { HomeContact } from "./HomeContact";
import { HomeGateways } from "./HomeGateways";
import { Screens } from "./Screens";
import { Source } from "./Source";
import { StatusStrip } from "./StatusStrip";

export default function Home() {
  useTitle("title.home");
  const ci = useGitHub(CI_PATH, "ci", parseCi);
  const commit = useGitHub(COMMIT_PATH, "commit", parseCommit);
  return (
    <>
      <StatusStrip ci={ci} />
      <Hero />
      <Screens />
      <HomeGateways />
      <About />
      <HomeContact />
      <Source ci={ci} commit={commit} />
    </>
  );
}
