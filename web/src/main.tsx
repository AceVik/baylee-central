import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { LangProvider, initialLang } from "./i18n";
import "./styles/base.css";

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
