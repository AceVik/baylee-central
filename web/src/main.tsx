import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { LangProvider, initialLang, loadLang, type Lang } from "./i18n";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/world.css";
import "./styles/header.css";
import "./styles/buttons.css";
import "./styles/forms.css";
import "./styles/footer.css";

const root = document.getElementById("root");
if (root) {
  // the first page is drawn in the chosen language at once: its words first
  // (English comes with the page; another language is a small file of its own)
  const chosen = initialLang();
  loadLang(chosen)
    .then(
      (): Lang => chosen,
      (): Lang => "en",
    )
    .then((initial) => {
      createRoot(root).render(
        <StrictMode>
          <LangProvider initial={initial}>
            <App />
          </LangProvider>
        </StrictMode>,
      );
    });
}
