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
  useTitle("title.download");
  const state = useRelease();
  return (
    <>
      <header className="page-head">
        <div className="section-inner">
          <p className="eyebrow">
            <T id="dlPage.eyebrow" />
          </p>
          <h1>
            <T id="dlPage.title" />
          </h1>
          <p className="section-lead">
            <T id="dlPage.lead" />
          </p>
          <div className="page-dl">
            <DownloadButton state={state} onDownloadPage />
          </div>
        </div>
      </header>

      <Section
        id="platforms"
        eyebrow={<T id="dlPage.filesEyebrow" />}
        title={<T id="dlPage.filesTitle" />}
      >
        <ReleaseHead release={state.release} />
        <Platforms state={state} />
      </Section>

      <InstallNotes />
    </>
  );
}
