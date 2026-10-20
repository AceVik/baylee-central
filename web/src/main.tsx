import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { LangProvider, initialLang } from "./i18n";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/header.css";
import "./styles/buttons.css";
import "./styles/forms.css";
import "./styles/footer.css";

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <LangProvider initial={initialLang()}>
        <App />
      </LangProvider>
    </StrictMode>,
  );
}
