// The account pages with the stand-in Central and crypto: registration to
// the recovery code (shown once, copied, downloaded, confirmed) and on to
// the account; what the form refuses before sending; the terms, accepted
// only when their texts match their digest; sign-in; the account page and
// the header.

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import type { TermsDocument } from "../../src/account/api";
import { termsDigest } from "../../src/account/digest";
import AccountPage from "../../src/account/AccountPage";
import RegisterPage from "../../src/account/RegisterPage";
import { ACCOUNT_KEY, AccountProvider } from "../../src/account/session";
import SignInPage from "../../src/account/SignInPage";
import { Header } from "../../src/components/Header";
import { LangProvider, type Lang } from "../../src/i18n";
import { CODE, KEY, fakeCentral, fakeCrypto, type FakeCentral } from "../support/central";

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

async function fill(user: ReturnType<typeof userEvent.setup>, fields: { key?: string; name?: string; password?: string; again?: string; age?: boolean; consent?: boolean } = {}) {
  const { key = KEY, name = "Synthetic_Name", password = PASSWORD, again = password, age = true, consent = true } = fields;
  if (key) await user.type(screen.getByLabelText("Beta key"), key);
  if (name) await user.type(screen.getByLabelText("Username"), name);
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

async function account(): Promise<FakeCentral> {
  const api = fakeCentral();
  const { register } = await import("../../src/account/flows");
  await register({ username: "synthetic_name", betaKey: KEY, password: PASSWORD }, { api, crypto: fakeCrypto(PASSWORD).factory });
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
    expect(api.accounts.has("synthetic_name")).toBe(true);
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
    expect(JSON.parse(window.localStorage.getItem(ACCOUNT_KEY) ?? "{}")).toMatchObject({ name: "synthetic_name" });
    expect(api.calls).toEqual(["register", "enrollment_start", "enrollment_finish", "discover", "login_start", "login_finish", "terms_status"]);
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
    expect(window.localStorage.getItem(ACCOUNT_KEY)).toBeNull();
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
    expect(await screen.findByText("synthetic_name")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Sign out" }));
    expect(await screen.findByText(/You are not signed in/)).toBeTruthy();
    expect(api.session).toBeNull();
    expect(window.localStorage.getItem(ACCOUNT_KEY)).toBeNull();
  });
});

describe("signing out", () => {
  it("says so when Central could not end the session, and when it ends by itself", async () => {
    window.localStorage.setItem(ACCOUNT_KEY, JSON.stringify({ name: "synthetic_name", expiresAt: Math.floor(Date.now() / 1000) + 600 }));
    const api = fakeCentral();
    api.session = { name: "synthetic_name", expiresAt: 4_000_000_000 };
    api.fail("session_status", "network");
    api.fail("logout", "network");
    const user = userEvent.setup();
    show(<AccountPage />, api);
    await user.click(await screen.findByRole("button", { name: "Sign out" }));
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", expect.stringMatching(/could not be reached .* until \d\d:\d\d/));
    expect(window.localStorage.getItem(ACCOUNT_KEY)).toBeNull();
  });
});

describe("the header", () => {
  it("offers to sign in, or names the account", async () => {
    show(<Header page="home" />, fakeCentral());
    expect(screen.getByRole("link", { name: "Sign in" }).getAttribute("href")).toBe("/signin");
  });

  it("names a signed-in account and links to it", () => {
    window.localStorage.setItem(ACCOUNT_KEY, JSON.stringify({ name: "synthetic_name", expiresAt: Math.floor(Date.now() / 1000) + 600 }));
    const api = fakeCentral();
    api.fail("session_status", "network");
    show(<Header page="account" />, api);
    const link = screen.getByRole("link", { name: "Your account, synthetic_name" });
    expect(link.getAttribute("href")).toBe("/account");
    expect(link.getAttribute("aria-current")).toBe("page");
  });
});
