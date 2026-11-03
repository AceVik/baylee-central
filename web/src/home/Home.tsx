// The start page:
// 1. StatusStrip (build, CI, commit, gateways) & Hero
// 2. Am Tisch (#screens)
// 3. Registrierte Gateways (#gateways)
// 4. Was es ist & KI-Integration (#about / #ai)
// 5. Kontakt (#contact)
// 6. Quelloffen (#source)

import { useMemo } from "react";

import { directoryClient } from "../central/directory";
import { CENTRAL_API, onLoopback } from "../config";
import { useDirectory } from "../gateways/useDirectory";
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
  // one read of Central's directory for the strip and the gateways section
  const client = useMemo(() => directoryClient({ api: CENTRAL_API, development: onLoopback() }), []);
  const directory = useDirectory(client);
  const listed = directory.loading && directory.gateways.length === 0 || directory.failure !== null ? null : directory.gateways;
  return (
    <>
      <StatusStrip ci={ci} commit={commit} gateways={listed} />
      <Hero />
      <Screens />
      <HomeGateways gateways={directory.gateways} loading={directory.loading} failure={directory.failure} retry={directory.retry} />
      <About />
      <HomeContact />
      <Source ci={ci} commit={commit} />
    </>
  );
}
