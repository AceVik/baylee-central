// How to run a gateway of one's own, from the old site's self-hosting part.

import { GITHUB } from "../config";
import { Section } from "../components/Section";
import { T } from "../i18n";

export function SelfHost() {
  return (
    <Section
      id="selfhost"
      alt
      eyebrow={<T en="Self-hosting" de="Selbst hosten" />}
      title={<T en="Run your own gateway" de="Ein eigenes Gateway betreiben" />}
    >
      <div className="two-col">
        <div>
          <p>
            <T
              en={
                <>
                  You do not need our server. A gateway is three kinds of process: <code>baylee-gateway</code> handles
                  accounts, decks, rooms and routing and needs PostgreSQL; <code>baylee-agent</code> starts one engine
                  process per game; <code>baylee-engine-server</code> is one game. The first two run no rules at all.
                </>
              }
              de={
                <>
                  Du brauchst unseren Server nicht. Ein Gateway sind drei Arten von Prozessen: <code>baylee-gateway</code>{" "}
                  verwaltet Konten, Decks, Räume und das Routing und braucht PostgreSQL; <code>baylee-agent</code> startet
                  pro Spiel einen Engine-Prozess; <code>baylee-engine-server</code> ist ein Spiel. Die ersten beiden
                  kennen keine Regeln.
                </>
              }
            />
          </p>
          <p>
            <T
              en={
                <>
                  Put a TLS proxy in front, set <code>BAYLEE_REGISTRATION=invite</code> if you want your own closed
                  circle, create keys with <code>baylee-gateway invite create</code>, and the client&apos;s front door
                  takes your address like any other. Card text in 19 languages is an optional ingest of Scryfall&apos;s
                  bulk data into the same database.
                </>
              }
              de={
                <>
                  Setz einen TLS-Proxy davor, setz <code>BAYLEE_REGISTRATION=invite</code>, wenn du einen eigenen
                  geschlossenen Kreis willst, erzeuge Schlüssel mit <code>baylee-gateway invite create</code>, und die
                  Eingangstür des Clients nimmt deine Adresse wie jede andere. Kartentext in 19 Sprachen ist ein
                  optionaler Import von Scryfalls Bulk-Daten in dieselbe Datenbank.
                </>
              }
            />
          </p>
          <p>
            <T
              en={
                <>
                  <a href={`${GITHUB}#running-your-own-gateway`} rel="noopener">
                    The self-hosting guide
                  </a>{" "}
                  walks through it;{" "}
                  <a href={`${GITHUB}/blob/main/docs/protocol.md`} rel="noopener">
                    docs/protocol.md
                  </a>{" "}
                  is the reference.
                </>
              }
              de={
                <>
                  <a href={`${GITHUB}/blob/main/README.de.md#ein-eigenes-gateway-betreiben`} rel="noopener">
                    Die Anleitung
                  </a>{" "}
                  geht alles durch;{" "}
                  <a href={`${GITHUB}/blob/main/docs/protocol.md`} rel="noopener">
                    docs/protocol.md
                  </a>{" "}
                  ist die Referenz.
                </>
              }
            />
          </p>
        </div>
        <div>
          <pre className="code" aria-label="Shell">
            <code>
              {`docker compose up -d
export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee
cargo build --release -p baylee-gateway -p baylee-agent -p baylee-engine-server

export BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32)
RUST_LOG=info BAYLEE_GATEWAY_NAME="My table" ./target/release/baylee-gateway
RUST_LOG=info ./target/release/baylee-agent`}
            </code>
          </pre>
          <p className="small">
            <T
              en={
                <>
                  Every gateway answers <code>GET /source</code> with its licence, version and commit, without sign-in.
                  The AGPL asks you to offer your source to the people who use your server; that route does it for you.
                </>
              }
              de={
                <>
                  Jedes Gateway beantwortet <code>GET /source</code> mit Lizenz, Version und Commit, ohne Anmeldung. Die
                  AGPL verlangt, dass du den Leuten, die deinen Server nutzen, den Quellcode anbietest; diese Route
                  erledigt das für dich.
                </>
              }
            />
          </p>
        </div>
      </div>
    </Section>
  );
}
