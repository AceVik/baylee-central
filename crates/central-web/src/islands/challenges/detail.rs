use super::{Hub, Lang, api, kind, play_link, text, words};
use leptos::prelude::*;

pub(super) fn view(lang: Lang, hub: Hub, task: &api::Challenge) -> impl IntoView + use<> {
    let id = task.id;
    let title = text(lang, &task.description.title);
    let instructions = text(lang, &task.description.instructions);
    let end = super::super::account::when(lang, task.description.period.ends_at);
    let cadence = kind(lang, task.description.period.kind);
    let withdrawn = task.withdrawn;
    let target = task.description.target_turn;
    let minutes = task.description.expected_minutes;
    let deck = task.description.deck.clone();
    view! {
        <button type="button" class="button button-ghost" disabled=move || hub.busy.get()
            on:click=move |_| hub.selected.set(None)>{words::admin_back(lang)}</button>
        <article class="card challenge-detail">
            <p class="eyebrow">{cadence}</p><h2>{title}</h2><p>{instructions}</p>
            {withdrawn.then(|| view! { <p role="status">{words::ch_withdrawn(lang)}</p> })}
            <dl class="challenge-stats">
                <div><dt>{words::ch_ends(lang)}</dt><dd class="num">{end}</dd></div>
                <div><dt>{words::ch_estimated(lang)}</dt><dd class="num">{format!("{minutes} min")}</dd></div>
                <div><dt>{words::ch_target(lang)}</dt><dd class="num">{target}</dd></div>
            </dl>
            {(!withdrawn).then(|| view! { <a class="button primary" href=play_link(id)>{words::ch_play(lang)}</a> })}
            <p class="small">{words::ch_play_help(lang)}" "<a href=format!("/{}/download",lang.code())>{words::nav_download(lang)}</a></p>
            <details><summary>{words::ch_deck(lang)}</summary><pre class="challenge-deck" translate="no">{deck}</pre></details>
            {personal(lang, hub, id, withdrawn)}
        </article>
        {rankings(lang, hub, task)}
    }
}

fn personal(lang: Lang, hub: Hub, id: [u8; 16], withdrawn: bool) -> impl IntoView {
    view! { {move || hub.personal.get().map(|own| view! {
        <section class="challenge-personal" aria-label=words::ch_progress(lang)>
            <h3>{words::ch_progress(lang)}</h3>
            <dl class="challenge-stats">
                <div><dt>{words::ch_attempts(lang)}</dt><dd class="num">{own.starts}</dd></div>
                <div><dt>{words::ch_failures(lang)}</dt><dd class="num">{own.failures}</dd></div>
                <div><dt>{words::ch_best_turns(lang)}</dt><dd class="num">{own.best_turn.map_or_else(|| "—".into(), |n| n.to_string())}</dd></div>
            </dl>
            {own.completed_at.is_some().then(|| view! { <p class="meta">{words::ch_completed(lang)}</p> })}
            { (own.hint_level > 0).then(|| view! { <p class="meta">{words::ch_assisted(lang)}</p> }) }
            <label class="challenge-consent" for="challenge-listing"><input id="challenge-listing" type="checkbox" prop:checked=own.listed disabled=move || hub.busy.get()
                on:change=move |ev| hub.change(api::Request::Visibility { challenge: id, listed: event_target_checked(&ev) },id) />{words::ch_consent(lang)}</label>
            {(!withdrawn).then(|| view! {
                <div class="challenge-actions">{[1,2].into_iter().map(move |level| view! {
                    <button type="button" class="button button-ghost" disabled=move || hub.busy.get() on:click=move |_| hub.confirm_hint.set(Some(level))>
                        {if level == 1 { words::ch_hint_one(lang) } else { words::ch_hint_two(lang) }}</button>
                }).collect_view()}</div>
            })}
            {move || hub.confirm_hint.get().map(|level| view! {
                <div class="challenge-hint-confirm" role="group" aria-label=words::ch_hint_confirm(lang)>
                    <p>{words::ch_hint_warning(lang)}</p>
                    <button type="button" class="button" on:click=move |_| hub.change(api::Request::Hint { challenge: id, level },id)>{words::ch_hint_confirm(lang)}</button>
                    <button type="button" class="button button-ghost" on:click=move |_| hub.confirm_hint.set(None)>{words::picture_cancel(lang)}</button>
                </div>
            })}
            {move || hub.hint.get().map(|hint| view! { <p class="challenge-hint" role="status">{text(lang,&hint)}</p> })}
        </section>
    })} }
}

fn board_label(lang: Lang, board: api::Board) -> &'static str {
    match board {
        api::Board::FirstWin => words::ch_first_win(lang),
        api::Board::BestTurns => words::ch_best_turns(lang),
        api::Board::Sprint => words::ch_sprint(lang),
    }
}

fn rankings(lang: Lang, hub: Hub, task: &api::Challenge) -> impl IntoView + use<> {
    let previous = task.clone();
    let next = task.clone();
    view! {
        <section class="card challenge-rankings" aria-labelledby="challenge-ranking-title">
            <h2 id="challenge-ranking-title">{words::ch_rankings(lang)}</h2>
            <p class="small">{words::ch_ranking_rules(lang)}</p>
            <div class="challenge-actions" role="group" aria-label=words::ch_rankings(lang)>
                {[api::Board::FirstWin,api::Board::BestTurns,api::Board::Sprint].into_iter().map(|board| {
                    let task = task.clone();
                    view! { <button type="button" class="button button-ghost" aria-pressed=move || (hub.board.get()==board).to_string()
                        disabled=move || hub.busy.get() on:click=move |_| hub.open(task.clone(),board,0)>{board_label(lang,board)}</button> }
                }).collect_view()}
            </div>
            {move || hub.rows.get().map(|rows| if rows.is_empty() {
                view! { <p class="empty-state">{words::ch_no_scores(lang)}</p> }.into_any()
            } else { table(lang,rows).into_any() })}
            <div class="challenge-actions">
                <button type="button" class="button button-ghost" disabled=move || hub.busy.get() || hub.offset.get()==0
                    on:click=move |_| hub.open(previous.clone(),hub.board.get_untracked(),hub.offset.get_untracked().saturating_sub(30))>{words::admin_back(lang)}</button>
                <button type="button" class="button button-ghost" disabled=move || { hub.busy.get() || hub.rows.with(|r| r.as_ref().is_none_or(|r| r.len()<30)) || hub.offset.get()>=3000 }
                    on:click=move |_| hub.open(next.clone(),hub.board.get_untracked(),hub.offset.get_untracked()+30)>{words::ch_more(lang)}</button>
            </div>
        </section>
    }
}

fn table(lang: Lang, rows: Vec<api::Standing>) -> impl IntoView {
    view! { <div class="challenge-table" tabindex="0" role="group" aria-label=words::ch_rankings(lang)><table>
        <thead><tr><th scope="col">{words::ch_rank(lang)}</th><th scope="col">{words::ch_player(lang)}</th>
            <th scope="col">{words::ch_failures(lang)}</th><th scope="col">{words::ch_turn(lang)}</th><th scope="col">{words::ch_time(lang)}</th></tr></thead>
        <tbody>{rows.into_iter().map(|row| {
            let name = row.tag.map_or_else(|| row.name.clone(), |tag| central_api::profile::handle(&row.name,tag));
            let time = row.milliseconds.map_or_else(|| "—".into(), |ms| format!("{}:{:02}.{:03}",ms/60000,ms/1000%60,ms%1000));
            view! { <tr class:challenge-own=row.own><td class="num">{row.rank}</td><th scope="row"><bdi translate="no">{name}</bdi></th>
                <td class="num">{row.failures}</td><td class="num">{row.turn}</td><td class="num">{time}</td></tr> }
        }).collect_view()}</tbody>
    </table></div> }
}
