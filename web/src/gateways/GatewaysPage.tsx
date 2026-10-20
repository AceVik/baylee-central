// The gateway directory: every gateway Baylee Central lists, read from its
// public `GatewayDirectory` call page by page, and how to run one of your
// own. Read-only; nothing here signs anybody in.

import { useMemo } from "react";

import { CENTRAL_API, onLoopback } from "../config";
import { directoryClient, type DirectoryClient } from "../central/directory";
import { T, useTitle } from "../i18n";
import "../styles/gateways.css";
import { Directory } from "./Directory";
import { SelfHost } from "./SelfHost";

export default function Gateways({ client }: { client?: DirectoryClient }) {
  useTitle("Gateways – Baylee", "Gateways – Baylee");
  const own = useMemo(() => client ?? directoryClient({ api: CENTRAL_API, development: onLoopback() }), [client]);
  return (
    <>
      <header className="page-head">
        <div className="section-inner">
          <p className="eyebrow">
            <T en="Online" de="Online" />
          </p>
          <h1>
            <T en="Gateways" de="Gateways" />
          </h1>
          <p className="section-lead">
            <T
              en="A gateway is a server where online tables live: accounts, decks, rooms and one engine process per game. These are the ones registered with Baylee Central, whose operator checked who runs each before listing it. Baylee runs Sanctuary; the others are run by other people for their groups, and their own rules apply there."
              de="Ein Gateway ist ein Server für Online-Tische: Konten, Decks, Räume und ein Engine-Prozess pro Spiel. Hier stehen die bei Baylee Central registrierten; dessen Betreiber hat bei jedem geprüft, wer es betreibt, bevor er es aufgenommen hat. Baylee betreibt Sanctuary; die anderen betreiben andere für ihre Runde, und dort gelten deren eigene Regeln."
            />
          </p>
          <p className="small">
            <T
              en="To play on one, copy its address into the client's front door. Online means Central heard from the gateway within the last minute and a half; it is no promise that it answers. Names and versions are what each operator reports."
              de="Um auf einem zu spielen, kopiere seine Adresse in die Eingangstür des Clients. Online heißt, Central hat in den letzten anderthalb Minuten von dem Gateway gehört; ein Versprechen, dass es antwortet, ist das nicht. Namen und Versionen sind die Angaben der Betreiber."
            />
          </p>
          <Directory client={own} />
        </div>
      </header>

      <SelfHost />
    </>
  );
}
