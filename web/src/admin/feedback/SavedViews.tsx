// An administrator's saved lists (central-api `ReportViews`): a name for the
// list's query string, kept by Central, so a list made on the laptop is
// there on the phone. Read when the menu first opens; a name saved again
// takes the new list.

import { useId, useState, type FormEvent } from "react";

import { ApiError } from "../../account/api";
import { idText } from "../../account/owner-api";
import { Icon } from "../../components/icons";
import { t, useLang } from "../../i18n";
import { Link } from "../../router";
import { failureKey } from "../shared";
import { BOUNDS, validView, type ReportsApi, type SavedView } from "./api";
import { useNotices } from "./pieces";
import { listPath, readQuery } from "./query";

export function SavedViews({ api, query }: { api: ReportsApi; query: string }) {
  const lang = useLang();
  const notices = useNotices();
  const id = useId();
  const [views, setViews] = useState<SavedView[] | null>(null);
  const [failed, setFailed] = useState(false);
  const [name, setName] = useState("");
  const read = () => {
    setFailed(false);
    api.views().then(setViews, () => setFailed(true));
  };
  const fail = (e: unknown) => notices.say(t(lang, failureKey(e instanceof ApiError ? e.kind : "network")), { bad: true });
  const save = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const chosen = name.trim();
    if (!validView(chosen, query)) return;
    api.saveView(chosen, query).then((all) => {
      setViews(all);
      setName("");
      notices.say(t(lang, "fb.viewSaved", { name: chosen }));
    }, fail);
  };
  const remove = (v: SavedView) =>
    api.deleteView(v.id).then((all) => {
      setViews(all);
      notices.say(t(lang, "fb.viewRemoved", { name: v.name }));
    }, fail);
  const current = views?.find((v) => v.query === query);
  const full = (views?.length ?? 0) >= BOUNDS.views;
  return (
    <details
      className="fb-menu"
      onToggle={(e) => {
        if (e.currentTarget.open && views === null) read();
      }}
    >
      <summary className="button button-ghost button-sm">
        <Icon name="bookmark" className="button-icon" />
        {t(lang, "fb.savedViews")}
      </summary>
      <div className="card fb-menu-body">
        {failed && (
          <p className="small" role="alert">
            {t(lang, "fb.errViews")}{" "}
            <button type="button" className="fb-linkish" onClick={read}>
              {t(lang, "gw.tryAgain")}
            </button>
          </p>
        )}
        {views !== null && views.length === 0 && <p className="small">{t(lang, "fb.noViews")}</p>}
        {views !== null && views.length > 0 && (
          <ul className="fb-view-list">
            {views.map((v) => (
              <li key={idText(v.id)}>
                <Link to={listPath(readQuery(v.query))} aria-current={v.query === query ? "true" : undefined}>
                  <bdi>{v.name}</bdi>
                </Link>
                <button type="button" className="round-btn fb-x" onClick={() => void remove(v)}>
                  <Icon name="close" />
                  <span className="vh">{t(lang, "fb.removeView", { name: v.name })}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
        {views !== null && current === undefined && (
          <form className="fb-inline" onSubmit={save}>
            <label className="vh" htmlFor={id}>
              {t(lang, "fb.viewName")}
            </label>
            <input id={id} className="fb-input" maxLength={BOUNDS.viewName} placeholder={t(lang, "fb.viewName")} value={name} disabled={full} onChange={(e) => setName(e.target.value)} />
            <button type="submit" className="button button-ghost button-sm" disabled={full || !validView(name.trim(), query)}>
              {t(lang, "fb.saveView")}
            </button>
          </form>
        )}
        {full && <p className="small">{t(lang, "fb.viewsFull", { n: BOUNDS.views })}</p>}
      </div>
    </details>
  );
}
