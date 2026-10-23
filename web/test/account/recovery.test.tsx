// The recovery code of an account moved here from the official gateway,
// with the stand-in Central and crypto: the account page's two wordings (and
// where the account came from), in English and German; the two ways to make
// the code (right after a sign-in, and the account page's button, which
// signs in again), each stored only once the person kept it; "Not now"; a
// lost answer retried with the same bytes; a code Central refuses.

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { describe, expect, it } from "vitest";

import type { TermsDocument } from "../../src/account/api";
import AccountPage from "../../src/account/AccountPage";
import { termsDigest } from "../../src/account/digest";
import { register } from "../../src/account/flows";
import { AccountProvider, SESSION_KEY } from "../../src/account/session";
import SignInPage from "../../src/account/SignInPage";
import { LangProvider, type Lang } from "../../src/i18n";
import { KEY, NEW_CODE, NEW_WRAPPER, fakeCentral, fakeCrypto, fill, type FakeCentral } from "../support/central";

const PASSWORD = "violet lantern harbour quietly";
const NAME = "moved_player";
const KEPT = "I have kept my recovery code in a safe place.";

function show(node: ReactNode, api: FakeCentral, lang: Lang = "en") {
  return render(
    <LangProvider initial={lang}>
      <AccountProvider api={api} crypto={fakeCrypto(PASSWORD).factory}>
        {node}
      </AccountProvider>
    </LangProvider>,
  );
}

/** A session this browser remembers and Central knows. */
function remembered(api: FakeCentral, name = NAME) {
  window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: 4_000_000_000 }));
  api.session = { name, expiresAt: 4_000_000_000 };
}

async function terms(): Promise<TermsDocument> {
  const texts = [
    { lang: "de", updated: "2026-10-22", markdown: "# Bedingungen\n\nSynthetische Bedingungen." },
    { lang: "en", updated: "2026-10-22", markdown: "# Terms\n\nSynthetic terms." },
  ];
  return { version: { version: "2026-10-22", digest: await termsDigest(texts) }, texts };
}

/** Signs in on `/signin` as the moved account, up to its new code. */
async function signInMoved(api: FakeCentral) {
  const user = userEvent.setup();
  show(<SignInPage />, api);
  await user.type(screen.getByLabelText("Username"), NAME);
  await user.type(screen.getByLabelText("Password"), PASSWORD);
  await user.click(screen.getByRole("button", { name: "Sign in" }));
  return user;
}

async function keep(user: ReturnType<typeof userEvent.setup>) {
  const step = await screen.findByRole("region", { name: "Your recovery code" });
  await user.click(within(step).getByLabelText(KEPT));
  await user.click(within(step).getByRole("button", { name: "Continue" }));
  return step;
}

describe("the account page", () => {
  it("says a moved account has no code yet, and offers to make one", async () => {
    const api = fakeCentral();
    api.moved(NAME);
    remembered(api);
    show(<AccountPage />, api);
    expect(
      await screen.findByText(
        "Your account has no recovery code yet. It moved here from the official gateway, which had none. If you forget your password without one, your encrypted data is lost.",
      ),
    ).toBeTruthy();
    expect(await screen.findByRole("button", { name: "Make a recovery code" })).toBeTruthy();
    expect(screen.queryByText(/You have a recovery code/)).toBeNull();
  });

  it("says so in German too", async () => {
    const api = fakeCentral();
    api.moved(NAME);
    remembered(api);
    show(<AccountPage />, api, "de");
    expect(
      await screen.findByText(
        "Dein Account hat noch keinen Wiederherstellungscode. Er ist vom offiziellen Gateway hierher umgezogen, das keinen kannte. Vergisst du ohne Code dein Passwort, sind deine verschlüsselten Daten verloren.",
      ),
    ).toBeTruthy();
    expect(await screen.findByRole("button", { name: "Wiederherstellungscode erstellen" })).toBeTruthy();
  });

  it("says an account has its code, and where it came from, in both languages", async () => {
    const api = fakeCentral();
    await register({ username: "synthetic_name", betaKey: KEY, password: PASSWORD }, { api, crypto: fakeCrypto(PASSWORD).factory });
    remembered(api, "synthetic_name");
    const { unmount } = show(<AccountPage />, api);
    expect(
      await screen.findByText(
        "You have a recovery code. With it, your encrypted data (settings, deck backups, chat history) survives a forgotten password, once password reset exists. Keep it where only you can reach it: we cannot show it again.",
      ),
    ).toBeTruthy();
    expect(screen.getByText("You created this account here, on baylee.app.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Make a recovery code" })).toBeNull();
    unmount();

    const moved = fakeCentral();
    moved.moved(NAME);
    const account = moved.accounts.get(NAME);
    if (account !== undefined) account.recovery = fill(73, 12);
    remembered(moved);
    show(<AccountPage />, moved, "de");
    expect(
      await screen.findByText(
        "Du hast einen Wiederherstellungscode. Mit ihm überstehen deine verschlüsselten Daten (Einstellungen, Deck-Sicherungen, Chatverlauf) ein vergessenes Passwort, sobald es das Zurücksetzen gibt. Bewahre ihn so auf, dass nur du ihn erreichst: Wir können ihn dir nicht noch einmal zeigen.",
      ),
    ).toBeTruthy();
    expect(screen.getByText("Dieser Account ist vom offiziellen Gateway hierher umgezogen.")).toBeTruthy();
  });

  it("makes the code by signing in again, and stores it once kept", async () => {
    const api = fakeCentral();
    api.moved(NAME);
    remembered(api);
    const user = userEvent.setup();
    show(<AccountPage />, api);
    await user.click(await screen.findByRole("button", { name: "Make a recovery code" }));
    expect(screen.getByText("To make the code, sign in again")).toBeTruthy();
    // a wrong password makes nothing
    await user.type(screen.getByLabelText("Password"), "wrong passphrase");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    expect(await screen.findByText("The username or the password is wrong.")).toBeTruthy();
    await user.clear(screen.getByLabelText("Password"));
    api.calls.length = 0;
    await user.type(screen.getByLabelText("Password"), PASSWORD);
    await user.click(screen.getByRole("button", { name: "Sign in" }));

    const step = await screen.findByRole("region", { name: "Your recovery code" });
    expect(within(step).getByLabelText("Recovery code").textContent).toBe(NEW_CODE);
    expect(within(step).getByText(/Here is a recovery code for your account, which has none yet\./)).toBeTruthy();
    expect(api.calls).toEqual(["discover", "login_start", "login_finish", "account_summary"]);
    expect(api.recoveries).toEqual([]);
    await keep(user);
    expect(await screen.findByText(/^You have a recovery code\./)).toBeTruthy();
    expect(screen.getByText("This account moved here from the official gateway.")).toBeTruthy();
    expect(api.recoveries).toEqual([NEW_WRAPPER]);
    expect(api.accounts.get(NAME)?.recovery).toEqual(NEW_WRAPPER);
  });
});

describe("right after a sign-in", () => {
  it("a moved account accepts the terms, then gets its code, stored only once kept", async () => {
    const api = fakeCentral({ terms: await terms() });
    api.moved(NAME);
    const user = await signInMoved(api);
    // the terms first: storing the code needs them accepted
    await user.click(await screen.findByRole("button", { name: "I accept" }));
    const step = await screen.findByRole("region", { name: "Your recovery code" });
    expect(within(step).getByLabelText("Recovery code").textContent).toBe(NEW_CODE);
    expect(api.recoveries).toEqual([]);
    await keep(user);
    await waitFor(() => expect(window.location.pathname).toBe("/account"));
    expect(api.recoveries).toEqual([NEW_WRAPPER]);
    expect(api.calls).toEqual(["discover", "login_start", "login_finish", "account_summary", "terms_status", "terms", "accept_terms", "set_recovery"]);
  });

  it("'Not now' throws the code away, and nothing reaches Central", async () => {
    const api = fakeCentral();
    api.moved(NAME);
    const user = await signInMoved(api);
    const step = await screen.findByRole("region", { name: "Your recovery code" });
    await user.click(within(step).getByRole("button", { name: "Not now" }));
    await waitFor(() => expect(window.location.pathname).toBe("/account"));
    expect(api.recoveries).toEqual([]);
    expect(api.calls).not.toContain("set_recovery");
    expect(api.accounts.get(NAME)?.recovery).toBeNull();
  });

  it("a registered account signs in without a new code", async () => {
    const api = fakeCentral();
    await register({ username: "synthetic_name", betaKey: KEY, password: PASSWORD }, { api, crypto: fakeCrypto(PASSWORD).factory });
    const user = userEvent.setup();
    show(<SignInPage />, api);
    await user.type(screen.getByLabelText("Username"), "synthetic_name");
    await user.type(screen.getByLabelText("Password"), PASSWORD);
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => expect(window.location.pathname).toBe("/account"));
    expect(screen.queryByRole("region", { name: "Your recovery code" })).toBeNull();
    expect(api.calls).not.toContain("set_recovery");
  });
});

describe("storing the code", () => {
  it("retries a lost answer with the same bytes, and no longer offers 'Not now'", async () => {
    const api = fakeCentral();
    api.moved(NAME);
    api.lose("set_recovery");
    const user = await signInMoved(api);
    const step = await keep(user);
    expect(await within(step).findByText(/Baylee Central did not answer, so the code may not be saved yet\./)).toBeTruthy();
    expect(within(step).queryByRole("button", { name: "Not now" })).toBeNull();
    expect(window.location.pathname).not.toBe("/account");
    await user.click(within(step).getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(window.location.pathname).toBe("/account"));
    expect(api.recoveries).toEqual([NEW_WRAPPER, NEW_WRAPPER]);
    expect(api.accounts.get(NAME)?.recovery).toEqual(NEW_WRAPPER);
  });

  it("says a code Central refuses does not count", async () => {
    const api = fakeCentral();
    api.moved(NAME);
    api.late = true;
    const user = await signInMoved(api);
    await keep(user);
    expect((await screen.findByRole("alert")).textContent).toMatch(/did not take this code, so it does not count: please throw it away/);
    expect(screen.queryByRole("region", { name: "Your recovery code" })?.querySelector("code")).toBeFalsy();
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(window.location.pathname).toBe("/account"));
    expect(api.accounts.get(NAME)?.recovery).toBeNull();
  });

  it("says a code is not the account's when another came first", async () => {
    const api = fakeCentral();
    api.moved(NAME);
    const user = await signInMoved(api);
    await screen.findByRole("region", { name: "Your recovery code" });
    // meanwhile, another tab stored its own
    const account = api.accounts.get(NAME);
    if (account !== undefined) account.recovery = fill(73, 12);
    await keep(user);
    expect((await screen.findByRole("alert")).textContent).toMatch(/has another recovery code by now, so this one does not count/);
    expect(api.accounts.get(NAME)?.recovery).toEqual(fill(73, 12));
  });
});
