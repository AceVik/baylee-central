use super::{Hub, Lang, api, standing_duration, standing_handle, words};
use leptos::prelude::*;

pub(super) fn view(lang: Lang, hub: Hub) -> impl IntoView {
    view! {
        <section class="card tower-rankings" aria-label=format!("{} · {}", words::tower_title(lang), words::ch_rankings(lang))>
            <h3 id="tower-rankings-title">{words::ch_rankings(lang)}</h3>
            <div class="challenge-actions">{
                [(api::Board::Ascent,words::tower_ascent(lang)),(api::Board::Sprint,words::tower_sprint(lang))].into_iter().map(|(board,title)| view! {
                    <button type="button" class="button button-ghost" aria-pressed=move || (hub.board.get() == board).to_string()
                        disabled=move || hub.busy.get() on:click=move |_| hub.board(board)>{title}</button>
                }).collect_view()
            }</div>
            <p class="meta">{words::tower_scoring(lang)}</p>
            <div class="challenge-table" tabindex="0" role="group" aria-label=words::ch_rankings(lang)
                aria-busy=move || hub.loading.get().then_some("true")
                hidden=move || hub.rows.with(Vec::is_empty)><table><thead><tr><th scope="col">{words::ch_rank(lang)}</th><th scope="col">{words::ch_player(lang)}</th>
                <th scope="col">{words::tower_floors(lang)}</th><th scope="col">{words::ch_failures(lang)}</th>
                <th scope="col">{words::ch_turn(lang)}</th><th scope="col">{words::ch_time(lang)}</th>
            </tr></thead><tbody>{move || hub.rows.get().into_iter().map(|row| {
                let name = standing_handle(&row.name, row.tag);
                let duration = standing_duration(row.milliseconds);
                view! { <tr class:challenge-own=row.own><td class="num">{words::number(lang, u64::from(row.rank))}</td><th scope="row"><bdi translate="no">{name}</bdi></th>
                    <td class="num">{row.floors}{" / 5"}</td><td class="num">{words::number(lang, u64::from(row.losses))}</td><td class="num">{words::number(lang, u64::from(row.turns))}</td><td class="num">{duration}</td>
                </tr> }
            }).collect_view()}</tbody></table></div>
            {move || hub.rows.with(Vec::is_empty).then(|| view! { <p class="empty-state">{words::ch_no_scores(lang)}</p> })}
            <div class="challenge-actions">
                <button type="button" class="button button-ghost" disabled=move || hub.busy.get() || hub.offset.get()==0
                    on:click=move |_| hub.page(false)>{words::shots_previous(lang)}</button>
                <button type="button" class="button button-ghost" disabled={move || hub.busy.get() || hub.rows.with(Vec::len)<30 || hub.offset.get()>=3000}
                    on:click=move |_| hub.page(true)>{words::shots_next(lang)}</button>
            </div>
        </section>
    }
}
