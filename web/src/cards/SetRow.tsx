// A row that opens: a set (its cards fetched when it opens) or an era of
// sets. What is inside exists only while the row is open.

import { useCallback, useEffect, useState, useTransition, type ReactNode } from "react";

import { T, num, tt, useLang } from "../i18n";
import { CardGrid } from "./CardGrid";
import { loadCards } from "./data";
import type { Card, Counts, SetSummary } from "./model";
import { CountsLine, Progress } from "./Progress";

function SetCards({ set }: { set: SetSummary }) {
  const lang = useLang();
  const [cards, setCards] = useState<Card[] | null | undefined>(undefined);
  const [, startTransition] = useTransition();
  useEffect(() => {
    let live = true;
    loadCards(set.code).then(
      (c) => live && startTransition(() => setCards(c)),
      () => live && setCards(null),
    );
    return () => {
      live = false;
    };
  }, [set.code]);
  if (cards === undefined) {
    return (
      <output className="small block">
        <T en="Loading…" de="Lädt …" />
      </output>
    );
  }
  if (cards === null) {
    return (
      <output className="small block">
        <T
          en="The card list did not load. Close and open the set to try again."
          de="Die Kartenliste wurde nicht geladen. Schließe das Set und öffne es erneut."
        />
      </output>
    );
  }
  return (
    <>
      <CardGrid cards={cards} counts={set} setName={set.name} />
      <p className="small set-foot">
        <a href={`https://scryfall.com/sets/${encodeURIComponent(set.code)}`} rel="noopener">
          {tt(lang, "This set on Scryfall", "Dieses Set bei Scryfall")}
        </a>
        {" · "}
        {tt(lang, "Images: Scryfall · © Wizards of the Coast", "Bilder: Scryfall · © Wizards of the Coast")}
      </p>
    </>
  );
}

/** A row that opens; `children` are rendered only while it is open. */
export function Row({
  id,
  className,
  open,
  onOpen,
  title,
  meta,
  counts,
  children,
}: {
  id?: string;
  className: string;
  open: boolean;
  onOpen: (open: boolean) => void;
  title: string;
  meta: ReactNode;
  counts: Counts;
  children: () => ReactNode;
}) {
  return (
    <details className={className} id={id} open={open} onToggle={(e) => onOpen(e.currentTarget.open)}>
      <summary className="set-sum">
        <span className="set-title">
          <span className="set-name">{title}</span>
          <span className="set-meta">{meta}</span>
        </span>
        <Progress counts={counts} />
        <CountsLine counts={counts} />
      </summary>
      <div className="set-body">{open && children()}</div>
    </details>
  );
}

export function SetRow({ set, withId, open, onOpen }: { set: SetSummary; withId?: boolean; open: boolean; onOpen: (code: string, open: boolean) => void }) {
  const lang = useLang();
  const year = set.released.slice(0, 4);
  return (
    <Row
      {...(withId === true ? { id: `set-${set.code}` } : {})}
      className="set"
      open={open}
      onOpen={(o) => onOpen(set.code, o)}
      title={set.name}
      meta={`${set.code.toUpperCase()}${year ? ` · ${year}` : ""} · ${num(lang, set.total)} ${set.total === 1 ? tt(lang, "card", "Karte") : tt(lang, "cards", "Karten")}`}
      counts={set}
    >
      {() => <SetCards set={set} />}
    </Row>
  );
}

export function useOpenSet(): [ReadonlySet<string>, (code: string, open: boolean) => void, (codes: string[]) => void] {
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  const toggle = useCallback(
    (code: string, o: boolean) =>
      setOpen((prev) => {
        if (prev.has(code) === o) return prev;
        const next = new Set(prev);
        if (o) next.add(code);
        else next.delete(code);
        return next;
      }),
    [],
  );
  const add = useCallback((codes: string[]) => setOpen((prev) => new Set([...prev, ...codes])), []);
  return [open, toggle, add];
}
