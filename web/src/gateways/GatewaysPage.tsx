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
  useTitle("title.gateways");
  const own = useMemo(() => client ?? directoryClient({ api: CENTRAL_API, development: onLoopback() }), [client]);
  return (
    <>
      <header className="page-head">
        <div className="section-inner">
          <p className="eyebrow">
            <T id="gw.eyebrow" />
          </p>
          <h1>
            <T id="gw.title" />
          </h1>
          <p className="section-lead">
            <T id="gw.lead" />
          </p>
          <p className="small">
            <T id="gw.howTo" />
          </p>
          <Directory client={own} />
        </div>
      </header>

      <SelfHost />
    </>
  );
}
