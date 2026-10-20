// First start on an unsigned build, checking a download, and what each
// system needs.

import { GITHUB, RELEASES_PAGE } from "../config";
import { Section } from "../components/Section";
import { T } from "../i18n";

export function InstallNotes() {
  return (
    <Section
      id="install"
      alt
      eyebrow={<T en="Installing" de="Installieren" />}
      title={<T en="First start and what it needs" de="Erster Start und was es braucht" />}
    >
      <div className="two-col">
        <div>
          <h3>
            <T en="First start" de="Erster Start" />
          </h3>
          <p>
            <T
              en={
                <>
                  The builds are not code-signed, because signing costs money. Windows and macOS therefore warn the
                  first time. On Windows, click <i>More info</i> and <i>Run anyway</i>. On macOS, open{" "}
                  <i>System Settings → Privacy &amp; Security</i> and click <i>Open Anyway</i>. The{" "}
                  <a href={`${GITHUB}#installing`} rel="noopener">
                    install notes
                  </a>{" "}
                  have every step and the Linux packages.
                </>
              }
              de={
                <>
                  Die Builds sind nicht signiert, weil Signieren Geld kostet. Windows und macOS warnen deshalb beim
                  ersten Mal. Unter Windows: <i>Weitere Informationen</i> und <i>Trotzdem ausführen</i>. Unter macOS:{" "}
                  <i>Systemeinstellungen → Datenschutz &amp; Sicherheit</i> und <i>Dennoch öffnen</i>. Die{" "}
                  <a href={`${GITHUB}/blob/main/README.de.md#installation`} rel="noopener">
                    Installationshinweise
                  </a>{" "}
                  haben jeden Schritt und die Linux-Pakete.
                </>
              }
            />
          </p>
          <h3>
            <T en="Checking a download" de="Einen Download prüfen" />
          </h3>
          <p>
            <T
              en={
                <>
                  Beside each file there is a <code>.sha256</code>; the archives also carry a <code>.sig</code>, the
                  Ed25519 signature the client checks before it installs an update by itself.{" "}
                  <code>sha256sum -c</code> on Linux, <code>shasum -a 256 -c</code> on macOS,{" "}
                  <code>certutil -hashfile … SHA256</code> on Windows.
                </>
              }
              de={
                <>
                  Neben jeder Datei liegt eine <code>.sha256</code>; die Archive tragen zusätzlich eine{" "}
                  <code>.sig</code>, die Ed25519-Signatur, die der Client prüft, bevor er ein Update selbst
                  installiert. <code>sha256sum -c</code> unter Linux, <code>shasum -a 256 -c</code> unter macOS,{" "}
                  <code>certutil -hashfile … SHA256</code> unter Windows.
                </>
              }
            />
          </p>
        </div>
        <div>
          <h3>
            <T en="What it needs" de="Was es braucht" />
          </h3>
          <ul className="plain">
            <li>
              <T
                en={
                  <>
                    <b>Windows</b> 10 or 11, x64 or ARM64. The x64 build needs a CPU with x86-64-v2 (SSE4.2), as
                    Windows 11 24H2 does.
                  </>
                }
                de={
                  <>
                    <b>Windows</b> 10 oder 11, x64 oder ARM64. Der x64-Build braucht eine CPU mit x86-64-v2 (SSE4.2),
                    wie Windows 11 24H2 auch.
                  </>
                }
              />
            </li>
            <li>
              <T
                en={
                  <>
                    <b>macOS</b> 11 or later on Apple silicon (M1 and later). There is no Intel build yet.
                  </>
                }
                de={
                  <>
                    <b>macOS</b> 11 oder neuer auf Apple Silicon (M1 und neuer). Einen Intel-Build gibt es noch nicht.
                  </>
                }
              />
            </li>
            <li>
              <T
                en={
                  <>
                    <b>Linux</b> with glibc 2.35 or newer (Ubuntu 22.04 and up), a Vulkan driver, and ALSA, udev,
                    X11/Wayland and xkbcommon; the <code>.deb</code> pulls them in.
                  </>
                }
                de={
                  <>
                    <b>Linux</b> mit glibc 2.35 oder neuer (ab Ubuntu 22.04), einem Vulkan-Treiber sowie ALSA, udev,
                    X11/Wayland und xkbcommon; das <code>.deb</code> zieht sie mit.
                  </>
                }
              />
            </li>
            <li>
              <T
                en={
                  <>
                    <b>Browser:</b> the browser version is paused for this beta and comes back later; until then, take
                    the desktop client above.
                  </>
                }
                de={
                  <>
                    <b>Browser:</b> Die Browser-Version pausiert in dieser Beta und kommt später wieder; bis dahin nimm
                    den Desktop-Client oben.
                  </>
                }
              />
            </li>
            <li>
              <T
                en={
                  <>
                    <b>Phones and tablets:</b> no builds to download yet. Android runs but cannot be used by touch;
                    iOS runs only in the simulator.
                  </>
                }
                de={
                  <>
                    <b>Handy und Tablet:</b> noch keine Builds zum Herunterladen. Android läuft, ist aber per Touch
                    nicht bedienbar; iOS läuft nur im Simulator.
                  </>
                }
              />
            </li>
          </ul>
          <p className="small">
            <a href={RELEASES_PAGE} rel="noopener">
              <T en="Every release on GitHub" de="Jede Version auf GitHub" />
            </a>
          </p>
        </div>
      </div>
    </Section>
  );
}
