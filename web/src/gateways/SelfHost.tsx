// How to run a gateway of one's own, from the old site's self-hosting part.

import { GITHUB } from "../config";
import { Link, PATHS } from "../router";
import { Section } from "../components/Section";
import { T, t, useLang, type Elems, type Lang } from "../i18n";

/** The self-hosting guide in the README of `lang`'s language, and the protocol's reference. */
const guides = (lang: Lang): Elems => ({
  guide: (words) => (
    <a href={`${GITHUB}${t(lang, "link.readmeGateway")}`} rel="noopener">
      {words}
    </a>
  ),
  protocol: (words) => (
    <a href={`${GITHUB}/blob/main/docs/protocol.md`} rel="noopener">
      {words}
    </a>
  ),
});

export function SelfHost() {
  const lang = useLang();
  return (
    <Section id="selfhost" alt eyebrow={<T id="selfhost.eyebrow" />} title={<T id="selfhost.title" />}>
      <div className="two-col">
        <div>
          <p>
            <T id="selfhost.processes" />
          </p>
          <p>
            <T id="selfhost.proxy" />
          </p>
          <p>
            <T id="selfhost.guide" elems={guides(lang)} />
          </p>
          <p>
            <T id="selfhost.central" elems={{ mine: (words) => <Link to={PATHS.accountGateways}>{words}</Link> }} />
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
            <T id="selfhost.source" />
          </p>
        </div>
      </div>
    </Section>
  );
}
