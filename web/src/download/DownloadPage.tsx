// The download page: the button for this machine, every platform's files,
// and how to install.

import { Section } from "../components/Section";
import { T, useTitle } from "../i18n";
import { DownloadButton } from "./DownloadButton";
import { InstallNotes } from "./InstallNotes";
import { Platforms } from "./Platforms";
import { ReleaseHead } from "./ReleaseHead";
import { useRelease } from "./useRelease";

export default function DownloadPage() {
  useTitle("Download – Baylee", "Herunterladen – Baylee");
  const state = useRelease();
  return (
    <>
      <header className="page-head">
        <div className="section-inner">
          <p className="eyebrow">
            <T en="Get it" de="Holen" />
          </p>
          <h1>
            <T en="Download" de="Herunterladen" />
          </h1>
          <p className="section-lead">
            <T
              en="Every download goes straight to the file on GitHub, nothing in between. Where a release has installers, they are the easy way; the archives hold the same game to unpack anywhere. An installed Baylee updates itself from the signed archives wherever it may write its own folder. Every package carries the seat bridge for language models beside the game."
              de="Jeder Download geht direkt zur Datei auf GitHub, nichts dazwischen. Wo eine Version Installer hat, sind sie der einfache Weg; die Archive enthalten dasselbe Spiel zum Entpacken an beliebiger Stelle. Ein installiertes Baylee aktualisiert sich selbst aus den signierten Archiven, wo es seinen eigenen Ordner beschreiben darf. Jedes Paket enthält neben dem Spiel die Sitzbrücke für Sprachmodelle."
            />
          </p>
          <div className="page-dl">
            <DownloadButton state={state} onDownloadPage />
          </div>
        </div>
      </header>

      <Section
        id="platforms"
        eyebrow={<T en="Every file" de="Jede Datei" />}
        title={<T en="All systems" de="Alle Systeme" />}
      >
        <ReleaseHead release={state.release} />
        <Platforms state={state} />
      </Section>

      <InstallNotes />
    </>
  );
}
