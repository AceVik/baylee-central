import { GITHUB } from "../config";
import { T } from "../i18n";
import { Section } from "../components/Section";

export function Opponents() {
  return (
    <Section
      id="ai"
      alt
      eyebrow={<T en="Opponents" de="Gegner" />}
      title={<T en="Opponents: the house, or a language model" de="Gegner: das Haus, oder ein Sprachmodell" />}
    >
      <div className="two-col reveal">
        <div>
          <h3>
            <T en="The house AI" de="Die Haus-KI" />
          </h3>
          <p>
            <T
              en="Five levels from novice to expert, available offline and at any online table. It plays from the same view a human gets, so hidden information stays hidden from it too. It is a heuristic player, not a trained one: good enough to learn a deck against, not a tournament opponent."
              de="Fünf Stufen von novice bis expert, offline und an jedem Online-Tisch. Sie spielt aus derselben Sicht wie ein Mensch, verdeckte Information bleibt also auch vor ihr verdeckt. Sie spielt heuristisch, nicht trainiert: gut genug, um ein Deck dagegen zu lernen, kein Turniergegner."
            />
          </p>
        </div>
        <div>
          <h3>
            <T en="A chair for a language model" de="Ein Stuhl für ein Sprachmodell" />
          </h3>
          <p>
            <T
              en="A seat bridge lets a model sit at the table as an ordinary player: a local model in LM Studio, one behind an OpenAI-compatible or Anthropic API, or an agent CLI on your own login. API keys never go in a file; a settings file names the environment variable they come from, and daily and monthly spending caps hold across games. When the model times out or answers something illegal, the house answers that question and the transcript says so."
              de="Eine Sitzbrücke lässt ein Modell als gewöhnlichen Spieler am Tisch sitzen: ein lokales Modell in LM Studio, eines hinter einer OpenAI-kompatiblen oder Anthropic-API oder ein Agent-CLI mit deinem eigenen Login. API-Schlüssel stehen nie in einer Datei; die Einstellungsdatei nennt die Umgebungsvariable, aus der sie kommen, und Tages- und Monatslimits gelten über alle Spiele. Wenn das Modell nicht antwortet oder etwas Unerlaubtes versucht, antwortet das Haus für diese Frage, und das Protokoll sagt es."
            />
          </p>
          <p>
            <T
              en={
                <>
                  At a gateway table the host seats one from the room: any open chair offers <i>language model</i> on a
                  desktop client, which starts the bridge (it ships beside the game) with your own settings and key.
                  Where the gateway&apos;s operator runs hosted models, a registered host can also pick one of those
                  instead, and it plays on the operator&apos;s machine, so no key or bridge is needed on yours.
                </>
              }
              de={
                <>
                  An einem Gateway-Tisch setzt der Gastgeber es im Raum hin: Jeder freie Stuhl bietet im Desktop-Client{" "}
                  <i>Sprachmodell</i> an, und der Client startet die Brücke (sie liegt dem Spiel bei) mit deinen eigenen
                  Einstellungen und deinem Schlüssel. Wo der Betreiber des Gateways gehostete Modelle anbietet, kann ein
                  registrierter Gastgeber stattdessen eines davon wählen; es spielt auf dem Rechner des Betreibers, auf
                  deinem braucht es dann weder Schlüssel noch Brücke.
                </>
              }
            />
          </p>
          <p className="small">
            <a href={`${GITHUB}#playing-against-a-language-model`} rel="noopener">
              <T en="How to set it up" de="So richtest du es ein (englisch)" />
            </a>
          </p>
        </div>
      </div>
    </Section>
  );
}
