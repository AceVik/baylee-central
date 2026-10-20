// The start page: what Baylee is, what it looks like, who plays against
// you, where online play stands and that it stays open source. The words are
// the old landing page's (Baylee's web/site/index.html), the layout new.

import { CI_PATH, COMMIT_PATH, parseCi, parseCommit, useGitHub } from "../github";
import { useTitle } from "../i18n";
import "../styles/home.css";
import { About } from "./About";
import { Hero } from "./Hero";
import { Online } from "./Online";
import { Opponents } from "./Opponents";
import { Screens } from "./Screens";
import { Source } from "./Source";
import { StatusStrip } from "./StatusStrip";

export default function Home() {
  useTitle("Baylee – a table that knows the rules", "Baylee – ein Tisch, der die Regeln kennt");
  const ci = useGitHub(CI_PATH, "ci", parseCi);
  const commit = useGitHub(COMMIT_PATH, "commit", parseCommit);
  return (
    <>
      <Hero />
      <StatusStrip ci={ci} commit={commit} />
      <About />
      <Screens />
      <Opponents />
      <Online />
      <Source ci={ci} commit={commit} />
    </>
  );
}
