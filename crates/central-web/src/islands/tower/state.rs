use super::{Failure, api, request, session};
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub(super) struct Hub {
    pub season: RwSignal<Option<api::Season>>,
    pub run: RwSignal<Option<api::Run>>,
    pub active: RwSignal<Option<api::Run>>,
    pub rows: RwSignal<Vec<api::Standing>>,
    pub badges: RwSignal<Vec<api::Badge>>,
    pub board: RwSignal<api::Board>,
    pub offset: RwSignal<u32>,
    pub busy: RwSignal<bool>,
    pub error: RwSignal<Option<Failure>>,
    pub signed_in: RwSignal<bool>,
    stamp: RwSignal<Option<i64>>,
    generation: RwSignal<u64>,
}

impl Hub {
    pub fn new(season: Option<api::Season>) -> Self {
        Self {
            season: RwSignal::new(season),
            run: RwSignal::new(None),
            active: RwSignal::new(None),
            rows: RwSignal::new(Vec::new()),
            badges: RwSignal::new(Vec::new()),
            board: RwSignal::new(api::Board::Ascent),
            offset: RwSignal::new(0),
            busy: RwSignal::new(true),
            error: RwSignal::new(None),
            signed_in: RwSignal::new(false),
            stamp: RwSignal::new(None),
            generation: RwSignal::new(0),
        }
    }

    fn clear_private(self) {
        update_run(self.run, None);
        update_run(self.active, None);
        if !self.badges.with_untracked(Vec::is_empty) {
            self.badges.set(Vec::new());
        }
        if self.signed_in.get_untracked() {
            self.signed_in.set(false);
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn session_changed(self) {
        self.generation
            .update(|generation| *generation = generation.wrapping_add(1));
        self.clear_private();
        self.stamp.set(None);
        self.busy.set(false);
        self.refresh();
    }

    pub fn refresh(self) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.error.set(None);
        let stamp = session::remembered();
        let generation = self.generation.get_untracked();
        if self.stamp.get_untracked() != stamp {
            self.clear_private();
            self.stamp.set(stamp);
        }
        let board = self.board.get_untracked();
        let offset = self.offset.get_untracked();
        leptos::task::spawn_local(async move {
            let response = request(api::Request::Read {
                season: None,
                board,
                offset,
            })
            .await;
            if self.generation.get_untracked() != generation {
                return;
            }
            if session::remembered() != stamp {
                self.clear_private();
                self.busy.set(false);
                return;
            }
            match response {
                Ok(api::Response::Read {
                    season,
                    standings,
                    run,
                    active,
                    badges,
                    ..
                }) => {
                    let season = season.map(|season| *season);
                    if self.season.get_untracked() != season {
                        self.season.set(season);
                    }
                    let same_rows = self.rows.with_untracked(|rows| {
                        rows.len() == standings.len()
                            && rows.iter().zip(&standings).all(|(a, b)| {
                                (
                                    a.rank,
                                    &a.name,
                                    a.tag,
                                    a.own,
                                    a.floors,
                                    a.losses,
                                    a.turns,
                                    a.milliseconds,
                                ) == (
                                    b.rank,
                                    &b.name,
                                    b.tag,
                                    b.own,
                                    b.floors,
                                    b.losses,
                                    b.turns,
                                    b.milliseconds,
                                )
                            })
                    });
                    if !same_rows {
                        self.rows.set(standings);
                    }
                    if stamp.is_some() {
                        if !self.signed_in.get_untracked() {
                            self.signed_in.set(true);
                        }
                        update_run(self.run, run.map(|run| *run));
                        update_run(self.active, active.map(|run| *run));
                        if self.badges.get_untracked() != badges {
                            self.badges.set(badges);
                        }
                    } else {
                        self.clear_private();
                    }
                }
                Ok(_) => self.error.set(Some(Failure::Malformed)),
                Err(error) => {
                    self.error.set(Some(error));
                    if error == Failure::Rejected {
                        self.clear_private();
                    }
                }
            }
            self.busy.set(false);
        });
    }

    pub fn board(self, board: api::Board) {
        if self.busy.get_untracked() {
            return;
        }
        self.board.set(board);
        self.offset.set(0);
        self.rows.set(Vec::new());
        self.refresh();
    }

    pub fn page(self, next: bool) {
        if self.busy.get_untracked() {
            return;
        }
        let offset = self.offset.get_untracked();
        self.offset.set(if next {
            (offset + 30).min(3000)
        } else {
            offset.saturating_sub(30)
        });
        self.refresh();
    }

    pub fn visibility(self, run: &api::Run) {
        if self.busy.get_untracked() {
            return;
        }
        let stamp = session::remembered();
        if stamp.is_none() || self.stamp.get_untracked() != stamp {
            self.clear_private();
            self.refresh();
            return;
        }
        self.busy.set(true);
        self.error.set(None);
        let season = run.season.id;
        let listed = !run.listed;
        let generation = self.generation.get_untracked();
        leptos::task::spawn_local(async move {
            let response = request(api::Request::Visibility { season, listed }).await;
            if self.generation.get_untracked() != generation {
                return;
            }
            if session::remembered() != stamp {
                self.clear_private();
                self.busy.set(false);
                return;
            }
            match response {
                Ok(api::Response::Updated) => {
                    self.busy.set(false);
                    self.refresh();
                }
                Ok(_) => {
                    self.error.set(Some(Failure::Malformed));
                    self.busy.set(false);
                }
                Err(error) => {
                    self.error.set(Some(error));
                    self.busy.set(false);
                }
            }
        });
    }
}

fn update_run(signal: RwSignal<Option<api::Run>>, run: Option<api::Run>) {
    let same = signal.with_untracked(|prior| match (prior, &run) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.id == b.id
                && a.revision == b.revision
                && a.season == b.season
                && a.attempt == b.attempt
                && a.listed == b.listed
        }
        _ => false,
    });
    if !same {
        signal.set(run);
    }
}
