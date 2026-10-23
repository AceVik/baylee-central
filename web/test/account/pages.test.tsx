// The account pages with the stand-in Central and crypto: registration to
// the recovery code (shown once, copied, downloaded, confirmed, with the
// handle drawn) and on to the account; what the form refuses before
// sending; the terms, accepted only when their texts match their digest;
// sign-in; the account page (the handle, the sign-in name, choosing a
// display name) and the header.

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import type { TermsDocument } from "../../src/account/api";
import { termsDigest } from "../../src/account/digest";
import AccountPage from "../../src/account/AccountPage";
import RegisterPage from "../../src/account/RegisterPage";
import { AccountProvider, SESSION_KEY } from "../../src/account/session";
import SignInPage from "../../src/account/SignInPage";
import { Header } from "../../src/components/Header";
import { LangProvider, type Lang } from "../../src/i18n";
import { CODE, KEY, fakeCentral, fakeCrypto, type FakeCentral } from "../support/central";

const HANDLE = "Synthetic_Name#3fa2";

const PASSWORD = "violet lantern harbour quietly";

function show(node: ReactNode, api: FakeCentral, lang: Lang = "en", password = PASSWORD) {
  return render(
    <LangProvider initial={lang}>
      <AccountProvider api={api} crypto={fakeCrypto(password).factory}>
        {node}
      </AccountProvider>
    </LangProvider>,
  );
}

async function fill(
  user: ReturnType<typeof userEvent.setup>,
  fields: { key?: string; name?: string; display?: string; password?: string; again?: string; age?: boolean; consent?: boolean } = {},
) {
  const { key = KEY, name = "Synthetic_Name", display = "Synthetic_Name", password = PASSWORD, again = password, age = true, consent = true } = fields;
  if (key) await user.type(screen.getByLabelText("Beta key"), key);
  if (name) await user.type(screen.getByLabelText("Username"), name);
  if (display) await user.type(screen.getByLabelText("Display name"), display);
  if (password) await user.type(screen.getByLabelText("Password"), password);
  if (again) await user.type(screen.getByLabelText("Password again"), again);
  if (age) await user.click(screen.getByLabelText("I am at least 16 years old."));
  if (consent) await user.click(screen.getByRole("checkbox", { name: /I accept the terms of use/ }));
}

async function terms(markdown = "# Terms\n\nSynthetic terms."): Promise<TermsDocument> {
  const texts = [
    { lang: "de", updated: "2026-10-21", markdown: "# Bedingungen\n\nSynthetische Bedingungen." },
    { lang: "en", updated: "2026-10-21", markdown },
  ];
  return { version: { version: "2026-10-21", digest: await termsDigest(texts) }, texts };
}

async function registerAndContinue(api: FakeCentral) {
  const user = userEvent.setup();
  show(<RegisterPage />, api);
  await fill(user);
  await user.click(screen.getByRole("button", { name: "Create account" }));
  await user.click(await screen.findByLabelText("I have kept my recovery code in a safe place."));
  await user.click(screen.getByRole("button", { name: "Continue" }));
  return user;
}

/** A registered account, `displayName` chosen at registration or (null) not yet. */
async function account(displayName: string | null = null, opts: Parameters<typeof fakeCentral>[0] = {}): Promise<FakeCentral> {
  const api = fakeCentral(opts);
  const { register } = await import("../../src/account/flows");
  await register({ username: "synthetic_name", betaKey: KEY, displayName, password: PASSWORD }, { api, crypto: fakeCrypto(PASSWORD).factory });
  return api;
}

describe("registration", () => {
  it("creates the account, shows the code once, and goes on to the account signed in", async () => {
    const api = fakeCentral();
    const user = userEvent.setup();
    const saved: Blob[] = [];
    URL.createObjectURL = vi.fn<(blob: Blob) => string>((blob) => {
      saved.push(blob);
      return "blob:synthetic";
    });
    URL.revokeObjectURL = vi.fn<(url: string) => void>();
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
    show(<RegisterPage />, api);
    await fill(user);
    await user.click(screen.getByRole("button", { name: "Create account" }));

    const step = await screen.findByRole("region", { name: "Your recovery code" });
    expect(within(step).getByLabelText("Recovery code").textContent).toBe(CODE);
    expect(api.accounts.get("synthetic_name")?.displayName).toBe("Synthetic_Name");
    // the whole handle, once the sign-in started at once has read the tag drawn
    expect((await within(step).findByText(HANDLE)).tagName).toBe("B");
    expect(within(step).getByText(/Other players see you as/)).toBeTruthy();
    await user.click(within(step).getByRole("button", { name: "Copy" }));
    expect(await navigator.clipboard.readText()).toBe(CODE);
    expect(await within(step).findByText("Copied.")).toBeTruthy();
    await user.click(within(step).getByRole("button", { name: "Download as a text file" }));
    expect(click).toHaveBeenCalledOnce();
    const file = await saved[0]?.text();
    expect(file).toContain(CODE);
    expect(file).toContain("synthetic_name");

    const go = within(step).getByRole("button", { name: "Continue" });
    expect(go).toHaveProperty("disabled", true);
    await user.click(within(step).getByLabelText("I have kept my recovery code in a safe place."));
    await user.click(go);
    await waitFor(() => expect(window.location.pathname).toBe("/account"));
    // only the end is kept; the name is Central's, in memory
    expect(JSON.parse(window.localStorage.getItem(SESSION_KEY) ?? "{}")).toEqual({ expiresAt: 4_000_000_000 });
    expect(api.calls).toEqual(["register", "enrollment_start", "enrollment_finish", "discover", "login_start", "login_finish", "account_summary", "terms_status"]);
  });

  it("says what is missing before sending anything", async () => {
    const api = fakeCentral();
    const user = userEvent.setup();
    show(<RegisterPage />, api);
    const submit = () => user.click(screen.getByRole("button", { name: "Create account" }));
    await submit();
    expect(screen.getByText("Please enter your beta key.")).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByLabelText("Beta key"));
    await user.type(screen.getByLabelText("Beta key"), KEY);
    await user.type(screen.getByLabelText("Username"), "a..b");
    await submit();
    expect(screen.getByText("That username does not follow the rules above.")).toBeTruthy();
    await user.clear(screen.getByLabelText("Username"));
    await user.type(screen.getByLabelText("Username"), "synthetic_name");
    await submit();
    expect(screen.getByText("That display name does not follow the rules above.")).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByLabelText("Display name"));
    for (const bad of ["_Synthetic", "ab", "Synthetic Name"]) {
      await user.clear(screen.getByLabelText("Display name"));
      await user.type(screen.getByLabelText("Display name"), bad);
      await submit();
      expect(screen.getByText("That display name does not follow the rules above.")).toBeTruthy();
    }
    await user.clear(screen.getByLabelText("Display name"));
    await user.type(screen.getByLabelText("Display name"), "Ad-min");
    await submit();
    expect(screen.getByText("That display name is kept for Baylee itself. Please choose another.")).toBeTruthy();
    await user.clear(screen.getByLabelText("Display name"));
    await user.type(screen.getByLabelText("Display name"), "Synthetic_Name");
    await user.type(screen.getByLabelText("Password"), "short");
    await submit();
    expect(screen.getByText(/needs at least 8 characters/)).toBeTruthy();
    expect(screen.getByText("Too short")).toBeTruthy();
    await user.type(screen.getByLabelText("Password"), " but now long enough");
    await user.type(screen.getByLabelText("Password again"), "something else");
    await submit();
    expect(screen.getByText("The two passwords differ.")).toBeTruthy();
    await waitFor(() => expect(document.activeElement).toBe(screen.getByLabelText("Password again")));
    await user.clear(screen.getByLabelText("Password again"));
    await user.type(screen.getByLabelText("Password again"), "short but now long enough");
    await submit();
    expect(screen.getByText("Please confirm that you are at least 16 years old.")).toBeTruthy();
    await user.click(screen.getByLabelText("I am at least 16 years old."));
    await submit();
    expect(screen.getByText("Please accept the terms of use.")).toBeTruthy();
    expect(api.calls).toEqual([]);
  });

  it("names a refused key and a taken name, in German too", async () => {
    const api = fakeCentral();
    const user = userEvent.setup();
    show(<RegisterPage />, api, "de");
    await user.type(screen.getByLabelText("Beta-Key"), "CENTRAL-WRONG");
    await user.type(screen.getByLabelText("Benutzername"), "synthetic_name");
    await user.type(screen.getByLabelText("Anzeigename"), "Synthetischer");
    await user.type(screen.getByLabelText("Passwort"), PASSWORD);
    await user.type(screen.getByLabelText("Passwort wiederholen"), PASSWORD);
    await user.click(screen.getByLabelText("Ich bin mindestens 16 Jahre alt."));
    await user.click(screen.getByRole("checkbox", { name: /Nutzungsbedingungen/ }));
    await user.click(screen.getByRole("button", { name: "Account anlegen" }));
    expect(await screen.findByText("Dieser Beta-Key ist ungültig, aufgebraucht oder abgelaufen.")).toBeTruthy();
    api.accounts.set("synthetic_name", { identity: { account: new Uint8Array(16), epoch: 1 }, wrapped: new Uint8Array(73), recovery: new Uint8Array(73) });
    await user.clear(screen.getByLabelText("Beta-Key"));
    await user.type(screen.getByLabelText("Beta-Key"), KEY);
    await user.click(screen.getByRole("button", { name: "Account anlegen" }));
    expect(await screen.findByText("Dieser Benutzername ist vergeben. Bitte wähl einen anderen.")).toBeTruthy();
  });
});

describe("the terms", () => {
  it("are shown in force and accepted for exactly their version and digest", async () => {
    const api = fakeCentral({ terms: await terms() });
    const user = await registerAndContinue(api);
    expect(await screen.findByRole("heading", { name: "Accept the terms of use" })).toBeTruthy();
    expect(screen.getByText("Synthetic terms.")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "I accept" }));
    await waitFor(() => expect(window.location.pathname).toBe("/account"));
    expect(api.accepted).toEqual(api.current?.version);
  });

  it("say so when declined: signed out, with the way back, and no dead button", async () => {
    const api = fakeCentral({ terms: await terms() });
    const user = await registerAndContinue(api);
    await user.click(await screen.findByRole("button", { name: "Not now (sign out)" }));
    expect(await screen.findByText(/You are signed out\./)).toBeTruthy();
    expect(screen.getByRole("link", { name: "Sign in again" }).getAttribute("href")).toBe("/signin");
    expect(screen.queryByRole("button", { name: "I accept" })).toBeNull();
    expect(api.session).toBeNull();
    expect(api.calls).not.toContain("accept_terms");
  });

  it("say so when the session ended before accepting", async () => {
    const api = fakeCentral({ terms: await terms() });
    const user = await registerAndContinue(api);
    await screen.findByRole("button", { name: "I accept" });
    api.session = null;
    await user.click(screen.getByRole("button", { name: "I accept" }));
    expect(await screen.findByText(/Your session has ended\./)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "I accept" })).toBeNull();
  });

  it("are not accepted when their texts do not hash to their digest", async () => {
    const doc = await terms();
    const [de, en] = doc.texts;
    if (de === undefined || en === undefined) throw new Error("two texts");
    const api = fakeCentral({ terms: { ...doc, texts: [de, { ...en, markdown: "# Terms\n\nOther terms." }] } });
    await registerAndContinue(api);
    expect(await screen.findByText(/do not match their fingerprint/)).toBeTruthy();
    expect(api.calls).not.toContain("accept_terms");
    expect(screen.queryByRole("button", { name: "I accept" })).toBeNull();
  });
});

describe("sign-in", () => {
  it("says a wrong password plainly and keeps nothing", async () => {
    const api = await account();
    const user = userEvent.setup();
    show(<SignInPage />, api);
    await user.type(screen.getByLabelText("Username"), "Synthetic_Name");
    await user.type(screen.getByLabelText("Password"), "wrong passphrase");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    expect(await screen.findByText("The username or the password is wrong.")).toBeTruthy();
    expect(api.session).toBeNull();
    expect(window.localStorage.getItem(SESSION_KEY)).toBeNull();
  });

  it("names the account as Central does, not as typed", async () => {
    const api = fakeCentral({ serverName: "central_name" });
    const { register } = await import("../../src/account/flows");
    await register({ username: "synthetic_name", betaKey: KEY, password: PASSWORD }, { api, crypto: fakeCrypto(PASSWORD).factory });
    const user = userEvent.setup();
    show(<SignInPage />, api);
    await user.type(screen.getByLabelText("Username"), "Synthetic_Name");
    await user.type(screen.getByLabelText("Password"), PASSWORD);
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => expect(window.location.pathname).toBe("/account"));
    expect(screen.queryByText(/central_name/)).toBeNull();
    expect(window.localStorage.getItem(SESSION_KEY)).not.toContain("name");
  });

  it("signs in and goes to the account, where signing out forgets it", async () => {
    const api = await account();
    const user = userEvent.setup();
    const { rerender } = show(<SignInPage />, api);
    await user.type(screen.getByLabelText("Username"), "synthetic_name");
    await user.type(screen.getByLabelText("Password"), PASSWORD);
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => expect(window.location.pathname).toBe("/account"));
    rerender(
      <LangProvider initial="en">
        <AccountProvider api={api} crypto={fakeCrypto(PASSWORD).factory}>
          <AccountPage />
        </AccountProvider>
      </LangProvider>,
    );
    // no display name yet: "signed in", and the sign-in name small, nowhere else
    expect(await screen.findByText("Sign-in name: synthetic_name")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Sign out" }));
    expect(await screen.findByText(/You are not signed in/)).toBeTruthy();
    expect(api.session).toBeNull();
    expect(window.localStorage.getItem(SESSION_KEY)).toBeNull();
  });
});

describe("signing out", () => {
  it("says so when Central could not end the session, and when it ends by itself", async () => {
    window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: Math.floor(Date.now() / 1000) + 600 }));
    const api = fakeCentral();
    api.moved("synthetic_name");
    api.session = { name: "synthetic_name", expiresAt: 4_000_000_000 };
    api.fail("account_summary", "network");
    api.fail("logout", "network");
    const user = userEvent.setup();
    show(<AccountPage />, api);
    await user.click(await screen.findByRole("button", { name: "Sign out" }));
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", expect.stringMatching(/could not be reached .* until \d\d:\d\d/));
    expect(window.localStorage.getItem(SESSION_KEY)).toBeNull();
  });
});

describe("the header", () => {
  it("offers to sign in, or names the account", async () => {
    show(<Header page="home" />, fakeCentral());
    expect(screen.getByRole("link", { name: "Sign in" }).getAttribute("href")).toBe("/signin");
  });

  it("names a remembered session as Central does, and links to it", async () => {
    window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: Math.floor(Date.now() / 1000) + 600 }));
    const api = await account("Synthetic_Name");
    api.session = { name: "synthetic_name", expiresAt: 4_000_000_000 };
    show(<Header page="account" />, api);
    // until Central answers, it is just "your account"
    expect(screen.getByRole("link", { name: "Your account" }).getAttribute("href")).toBe("/account");
    // the handle others see, never the private sign-in name
    const link = await screen.findByRole("link", { name: `Your account, ${HANDLE}` });
    expect(link.textContent).toBe(HANDLE);
    expect(link.getAttribute("href")).toBe("/account");
    expect(link.getAttribute("aria-current")).toBe("page");
  });

  it("stays unnamed while Central cannot be reached", async () => {
    window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: Math.floor(Date.now() / 1000) + 600 }));
    const api = fakeCentral();
    api.fail("account_summary", "network");
    show(<Header page="home" />, api);
    await waitFor(() => expect(api.calls).toEqual(["account_summary"]));
    expect(screen.getByRole("link", { name: "Your account" }).getAttribute("href")).toBe("/account");
  });
});

/** Signed in to `api`'s account on the account page, with the header. */
async function signedIn(api: FakeCentral) {
  const { signIn } = await import("../../src/account/flows");
  await signIn({ username: "synthetic_name", password: PASSWORD }, { api, crypto: fakeCrypto(PASSWORD).factory });
  window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: 4_000_000_000 }));
  api.calls.length = 0;
  const user = userEvent.setup();
  show(
    <>
      <Header page="account" />
      <AccountPage />
    </>,
    api,
  );
  return user;
}

describe("the display name", () => {
  it("shows the handle on the account page, the sign-in name small", async () => {
    const api = await account("Synthetic_Name");
    await signedIn(api);
    expect(await screen.findByText(HANDLE, { selector: ".account-name bdi" })).toBeTruthy();
    expect(screen.getByText("Sign-in name: synthetic_name")).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Choose your display name" })).toBeNull();
  });

  it("is chosen first on the account page by an account without one, within the rules, once", async () => {
    const api = await account(null);
    const user = await signedIn(api);
    const form = await screen.findByRole("region", { name: "Choose your display name" });
    expect(within(form).getByText(/beside your tag, #3fa2/)).toBeTruthy();
    expect(screen.getByRole("link", { name: "Your account" })).toBeTruthy();
    const save = () => user.click(within(form).getByRole("button", { name: "Save the name" }));
    await user.type(within(form).getByLabelText("Display name"), "System");
    await save();
    expect(within(form).getByText("That display name is kept for Baylee itself. Please choose another.")).toBeTruthy();
    await user.clear(within(form).getByLabelText("Display name"));
    await user.type(within(form).getByLabelText("Display name"), "-Chosen");
    await save();
    expect(within(form).getByText("That display name does not follow the rules above.")).toBeTruthy();
    expect(api.names).toEqual([]);
    await user.clear(within(form).getByLabelText("Display name"));
    await user.type(within(form).getByLabelText("Display name"), "Chosen_Name");
    await save();
    // the header and the page show it at once
    expect(await screen.findByRole("link", { name: "Your account, Chosen_Name#3fa2" })).toBeTruthy();
    expect(screen.getByText("Chosen_Name#3fa2", { selector: ".account-name bdi" })).toBeTruthy();
    expect(screen.queryByRole("region", { name: "Choose your display name" })).toBeNull();
    expect(api.names).toEqual(["Chosen_Name"]);
  });

  it("shows a name chosen elsewhere meanwhile, and keeps it", async () => {
    const api = await account(null);
    const user = await signedIn(api);
    const form = await screen.findByRole("region", { name: "Choose your display name" });
    const stored = api.accounts.get("synthetic_name");
    if (stored === undefined) throw new Error("the account");
    stored.displayName = "Elsewhere";
    await user.type(within(form).getByLabelText("Display name"), "Chosen_Name");
    await user.click(within(form).getByRole("button", { name: "Save the name" }));
    expect(await screen.findByText("Elsewhere#3fa2", { selector: ".account-name bdi" })).toBeTruthy();
    expect(screen.queryByRole("region", { name: "Choose your display name" })).toBeNull();
  });

  it("waits for the terms, which choosing a name needs accepted", async () => {
    const api = await account(null, { terms: await terms() });
    const user = await signedIn(api);
    await user.click(await screen.findByRole("button", { name: "I accept" }));
    expect(await screen.findByRole("region", { name: "Choose your display name" })).toBeTruthy();
  });

  it("comes from the official gateway for a moved account, in German too", async () => {
    const api = fakeCentral();
    api.moved("synthetic_name");
    const { signIn } = await import("../../src/account/flows");
    await signIn({ username: "synthetic_name", password: PASSWORD }, { api, crypto: fakeCrypto(PASSWORD).factory });
    window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: 4_000_000_000 }));
    show(<AccountPage />, api, "de");
    expect(await screen.findByText(/kommen vom offiziellen Gateway mit/)).toBeTruthy();
    expect(screen.getByText("Anmeldename: synthetic_name")).toBeTruthy();
    expect(screen.queryByRole("region", { name: "Wähl deinen Anzeigenamen" })).toBeNull();
  });

  it("is a moved account's own once carried", async () => {
    const api = fakeCentral();
    api.moved("synthetic_name", { displayName: "Carried", tag: 13 });
    const { signIn } = await import("../../src/account/flows");
    await signIn({ username: "synthetic_name", password: PASSWORD }, { api, crypto: fakeCrypto(PASSWORD).factory });
    window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: 4_000_000_000 }));
    show(<AccountPage />, api);
    expect(await screen.findByText("Carried#000d", { selector: ".account-name bdi" })).toBeTruthy();
    expect(screen.queryByText(/come with you from the official gateway/)).toBeNull();
  });
});
