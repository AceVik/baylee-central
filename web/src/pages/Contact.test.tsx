import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { LangProvider } from "../i18n";
import { serve } from "../test-utils";
import { ContactForm } from "./Contact";

const ENDPOINT = "https://feedback.example.invalid/site/contact";

function form(lang: "en" | "de" = "en") {
  return render(
    <LangProvider initial={lang}>
      <ContactForm endpoint={ENDPOINT} />
    </LangProvider>,
  );
}

describe("the contact form", () => {
  it("sends exactly the documented fields, without credentials, and thanks", async () => {
    const calls = serve(() => ({ status: 201, body: { ok: true } }));
    const user = userEvent.setup();
    form("de");
    await user.type(screen.getByLabelText(/Nachricht/), "  Hallo, eine Frage.  ");
    await user.click(screen.getByRole("button", { name: "Senden" }));
    await waitFor(() => expect(screen.getByText("Danke, sie ist unterwegs.")).toBeTruthy(), { timeout: 2000 });
    expect(calls).toHaveLength(1);
    expect(calls[0]?.url).toBe(ENDPOINT);
    expect(calls[0]?.init?.credentials).toBe("omit");
    expect(calls[0]?.body).toEqual({ name: null, email: null, message: "Hallo, eine Frage.", language: "de", website: "" });
  });

  it("asks for a message and a plausible address before sending", async () => {
    const calls = serve(() => ({ status: 201 }));
    const user = userEvent.setup();
    form();
    await user.click(screen.getByRole("button", { name: "Send" }));
    expect(screen.getByText("Please write a message first.")).toBeTruthy();
    expect(screen.getByLabelText(/Message/).getAttribute("aria-invalid")).toBe("true");
    await user.type(screen.getByLabelText(/Message/), "Hi");
    await user.type(screen.getByLabelText(/E-mail/), "not an address");
    await user.click(screen.getByRole("button", { name: "Send" }));
    expect(screen.getByText(/does not look right/)).toBeTruthy();
    expect(calls).toHaveLength(0);
  });

  it("says why a message was not taken", async () => {
    serve(() => ({ status: 429, text: "" }));
    const user = userEvent.setup();
    form();
    await user.type(screen.getByLabelText(/Message/), "Hi");
    await user.click(screen.getByRole("button", { name: "Send" }));
    expect(await screen.findByText(/Too many messages from here/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Try again" })).toBeTruthy();
  });
});
