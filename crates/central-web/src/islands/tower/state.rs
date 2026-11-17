use super::{Failure, api, progress, request, session};
use crate::islands::medals;
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub(super) struct Hub {
    pub season: RwSignal<Option<api::Season>>,
    pub run: RwSignal<Option<api::Run>>,
    pub active: RwSignal<Option<api::Run>>,
    pub rows: RwSignal<Vec<api::Standing>>,
    pub badges: RwSignal<Vec<api::Badge>>,
    pub selected: RwSignal<Option<medals::Preview>>,
    pub board: RwSignal<api::Board>,
    pub offset: RwSignal<u32>,
    pub busy: RwSignal<bool>,
    pub loading: RwSignal<bool>,
    pub error: RwSignal<Option<Failure>>,
    pub signed_in: RwSignal<bool>,
    in_flight: RwSignal<bool>,
    stamp: RwSignal<Option<String>>,
    generation: RwSignal<u64>,
}

impl Hub {
    pub fn new(season: Option<api::Season>, standings: Vec<api::Standing>) -> Self {
        Self {
            season: RwSignal::new(season),
            run: RwSignal::new(None),
            active: RwSignal::new(None),
            rows: RwSignal::new(standings),
            badges: RwSignal::new(Vec::new()),
            selected: RwSignal::new(None),
            board: RwSignal::new(api::Board::Ascent),
            offset: RwSignal::new(0),
            busy: RwSignal::new(true),
            loading: RwSignal::new(false),
            error: RwSignal::new(None),
            signed_in: RwSignal::new(false),
            in_flight: RwSignal::new(false),
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
        self.selected.set(None);
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
        self.loading.set(false);
        self.in_flight.set(false);
        self.refresh(false);
    }

    pub fn refresh(self, interactive: bool) {
        if interactive {
            if self.busy.get_untracked() {
                return;
            }
            self.generation
                .update(|generation| *generation = generation.wrapping_add(1));
            self.busy.set(true);
            self.loading.set(true);
        } else if self.busy.get_untracked() || self.in_flight.get_untracked() {
            return;
        }
        self.in_flight.set(true);
        self.error.set(None);
        let stamp = session::marker();
        let generation = self.generation.get_untracked();
        if self.stamp.get_untracked() != stamp {
            self.clear_private();
            self.stamp.set(stamp.clone());
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
            if session::marker() != stamp {
                self.clear_private();
                self.busy.set(false);
                self.loading.set(false);
                self.in_flight.set(false);
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
                    let changed = self
                        .season
                        .with_untracked(|prior| prior.as_ref().map(|s| s.id))
                        != season.as_ref().map(|s| s.id);
                    if self.season.get_untracked() != season {
                        self.season.set(season);
                    }
                    if changed && offset > 0 {
                        self.offset.set(0);
                        self.busy.set(false);
                        self.loading.set(false);
                        self.in_flight.set(false);
                        self.refresh(interactive);
                        return;
                    }
                    update_rows(self.rows, standings);
                    if stamp.is_some() {
                        if !self.signed_in.get_untracked() {
                            self.signed_in.set(true);
                        }
                        update_run(self.run, run.map(|run| *run));
                        update_run(self.active, active.map(|run| *run));
                        if self.badges.get_untracked() != badges {
                            self.selected.update(|selected| {
                                if let Some(preview) = selected
                                    && let Some(updated) = badges
                                        .iter()
                                        .find(|b| progress::motif_of(b.kind) == preview.motif)
                                {
                                    preview.count = updated.count;
                                }
                            });
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
                        session::forget();
                        self.stamp.set(None);
                        self.clear_private();
                    }
                }
            }
            self.busy.set(false);
            self.loading.set(false);
            self.in_flight.set(false);
        });
    }

    pub fn board(self, board: api::Board) {
        if self.busy.get_untracked() {
            return;
        }
        self.board.set(board);
        self.offset.set(0);
        self.refresh(true);
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
        self.refresh(true);
    }

    pub fn visibility(self, season: [u8; 16], listed: bool, checked: RwSignal<bool>) {
        let prior = self
            .active
            .with_untracked(|r| {
                r.as_ref()
                    .filter(|r| r.season.id == season)
                    .map(|r| r.listed)
            })
            .or_else(|| {
                self.run.with_untracked(|r| {
                    r.as_ref()
                        .filter(|r| r.season.id == season)
                        .map(|r| r.listed)
                })
            })
            .unwrap_or(!listed);
        if self.busy.get_untracked() {
            checked.try_set(prior);
            return;
        }
        let stamp = session::marker();
        if stamp.is_none() || self.stamp.get_untracked() != stamp {
            checked.try_set(prior);
            self.clear_private();
            self.refresh(false);
            return;
        }
        self.generation
            .update(|generation| *generation = generation.wrapping_add(1));
        self.busy.set(true);
        self.loading.set(true);
        self.in_flight.set(true);
        self.error.set(None);
        let generation = self.generation.get_untracked();
        leptos::task::spawn_local(async move {
            let response = request(api::Request::Visibility { season, listed }).await;
            if self.generation.get_untracked() != generation {
                return;
            }
            if session::marker() != stamp {
                self.clear_private();
                self.busy.set(false);
                self.loading.set(false);
                self.in_flight.set(false);
                return;
            }
            match response {
                Ok(api::Response::Updated) => {
                    // The mutation is acknowledged even if the subsequent Read fails.
                    // Rollbacks must use this last confirmed value, never a stale run.
                    for signal in [self.run, self.active] {
                        signal.update_untracked(|run| {
                            if let Some(run) = run
                                && run.season.id == season
                            {
                                run.listed = listed;
                            }
                        });
                    }
                    self.busy.set(false);
                    self.loading.set(false);
                    self.in_flight.set(false);
                    self.refresh(false);
                }
                Ok(_) => {
                    checked.try_set(prior);
                    self.error.set(Some(Failure::Malformed));
                    self.busy.set(false);
                    self.loading.set(false);
                    self.in_flight.set(false);
                }
                Err(error) => {
                    checked.try_set(prior);
                    self.error.set(Some(error));
                    if error == Failure::Rejected {
                        session::forget();
                        self.stamp.set(None);
                        self.clear_private();
                    }
                    self.busy.set(false);
                    self.loading.set(false);
                    self.in_flight.set(false);
                }
            }
        });
    }
}

fn update_rows(signal: RwSignal<Vec<api::Standing>>, standings: Vec<api::Standing>) {
    let same = signal.with_untracked(|rows| {
        rows.len() == standings.len()
            && rows
                .iter()
                .zip(&standings)
                .all(|(a, b)| crate::data::standing_eq(a, b))
    });
    if !same {
        signal.set(standings);
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
