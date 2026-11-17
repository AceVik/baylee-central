use super::{ChallengeFeed, Failure, api, progress, request, session};
use crate::islands::medals;
use leptos::prelude::*;

#[derive(Clone, Default)]
struct SyncState {
    loading: bool,
    in_flight: bool,
    stamp: Option<String>,
    generation: u64,
    page: Option<i64>,
    return_to: Option<[u8; 16]>,
}

#[derive(Clone, Copy)]
pub(super) struct Hub {
    pub feed: RwSignal<Option<ChallengeFeed>>,
    pub selected: RwSignal<Option<api::Challenge>>,
    pub board: RwSignal<api::Board>,
    pub offset: RwSignal<u32>,
    pub rows: RwSignal<Option<Vec<api::Standing>>>,
    pub personal: RwSignal<Option<api::TaskProgress>>,
    pub badges: RwSignal<Vec<api::Badge>>,
    pub active: RwSignal<Vec<api::Attempt>>,
    pub selected_medal: RwSignal<Option<medals::Preview>>,
    pub signed_in: RwSignal<bool>,
    pub busy: RwSignal<bool>,
    pub error: RwSignal<Option<Failure>>,
    pub hint: RwSignal<Option<api::Words>>,
    pub confirm_hint: RwSignal<Option<u8>>,
    sync: RwSignal<SyncState>,
}

impl Hub {
    pub fn new(initial: Option<ChallengeFeed>) -> Self {
        Self {
            feed: RwSignal::new(initial),
            selected: RwSignal::new(None),
            board: RwSignal::new(api::Board::FirstWin),
            offset: RwSignal::new(0),
            rows: RwSignal::new(None),
            personal: RwSignal::new(None),
            badges: RwSignal::new(vec![]),
            active: RwSignal::new(vec![]),
            selected_medal: RwSignal::new(None),
            signed_in: RwSignal::new(false),
            // Server-rendered controls cannot act until hydration attaches their handlers.
            busy: RwSignal::new(true),
            error: RwSignal::new(None),
            hint: RwSignal::new(None),
            confirm_hint: RwSignal::new(None),
            sync: RwSignal::new(SyncState::default()),
        }
    }

    pub fn is_loading(self) -> bool {
        self.sync.with(|s| s.loading)
    }

    pub fn page_cursor(self) -> Option<i64> {
        self.sync.with_untracked(|s| s.page)
    }

    pub fn return_target(self) -> Option<[u8; 16]> {
        self.sync.with_untracked(|s| s.return_to)
    }

    fn generation(self) -> u64 {
        self.sync.with_untracked(|s| s.generation)
    }

    fn clear_private(self) {
        if self.signed_in.get_untracked() {
            self.signed_in.set(false);
        }
        if !self.badges.with_untracked(Vec::is_empty) {
            self.badges.set(vec![]);
        }
        if !self.active.with_untracked(Vec::is_empty) {
            self.active.set(vec![]);
        }
        self.selected_medal.set(None);
        self.personal.set(None);
        self.hint.set(None);
        self.confirm_hint.set(None);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn session_changed(self) {
        let page = self.page_cursor();
        self.sync.update(|s| {
            s.generation = s.generation.wrapping_add(1);
            s.stamp = None;
            s.loading = false;
            s.in_flight = false;
        });
        self.clear_private();
        self.busy.set(false);
        self.refresh(page, false);
    }

    fn begin(self, interactive: bool) -> Option<u64> {
        if interactive {
            if self.busy.get_untracked() {
                return None;
            }
            let mut started = 0;
            self.sync.update(|s| {
                s.generation = s.generation.wrapping_add(1);
                s.loading = true;
                s.in_flight = true;
                started = s.generation;
            });
            self.busy.set(true);
            self.error.set(None);
            Some(started)
        } else {
            if self.busy.get_untracked() || self.sync.with_untracked(|s| s.in_flight) {
                return None;
            }
            let mut started = 0;
            self.sync.update_untracked(|s| {
                s.in_flight = true;
                started = s.generation;
            });
            self.error.set(None);
            Some(started)
        }
    }

    fn finish(self, started: u64) {
        if self.generation() == started {
            self.busy.set(false);
            self.sync.update(|s| {
                s.loading = false;
                s.in_flight = false;
            });
        }
    }

    pub fn refresh(self, before: Option<i64>, interactive: bool) {
        let Some(started) = self.begin(interactive) else {
            return;
        };
        self.sync.update_untracked(|s| s.page = before);
        let stamp = session::marker();
        if self.sync.with_untracked(|s| s.stamp != stamp) {
            self.clear_private();
            self.sync.update_untracked(|s| s.stamp.clone_from(&stamp));
        }
        leptos::task::spawn_local(async move {
            let result = request(api::Request::Catalogue { before }).await;
            if self.generation() != started {
                return;
            }
            match result {
                Ok(api::Response::Catalogue {
                    now,
                    challenges,
                    before,
                }) => {
                    // A clock tick alone must not replace focused calendar controls.
                    let changed = self.feed.with_untracked(|feed| {
                        feed.as_ref().is_none_or(|old| {
                            old.before != before
                                || old.challenges != challenges
                                || challenges.iter().any(|task| {
                                    task.description.period.live_at(old.now)
                                        != task.description.period.live_at(now)
                                })
                        })
                    });
                    let feed = Some(ChallengeFeed {
                        tower: None,
                        tower_standings: Vec::new(),
                        now,
                        challenges,
                        before,
                    });
                    if changed {
                        self.feed.set(feed);
                    } else {
                        self.feed.update_untracked(|current| *current = feed);
                    }
                }
                Ok(_) => self.error.set(Some(Failure::Malformed)),
                Err(error) => self.error.set(Some(error)),
            }
            self.load_progress(started, stamp.as_deref()).await;
            if self.generation() != started {
                return;
            }
            if let Some(task) = self.selected.get_untracked() {
                self.load_detail(
                    started,
                    task.id,
                    self.board.get_untracked(),
                    self.offset.get_untracked(),
                )
                .await;
                if self.generation() != started {
                    return;
                }
                self.load_personal(started, stamp.as_deref(), task.id).await;
            }
            self.finish(started);
        });
    }

    async fn load_progress(self, started: u64, stamp: Option<&str>) {
        let Some(expected) = stamp else {
            self.clear_private();
            return;
        };
        let result = request(api::Request::Progress).await;
        if self.generation() != started {
            return;
        }
        if session::marker().as_deref() != Some(expected) {
            self.clear_private();
            return;
        }
        match result {
            Ok(api::Response::Progress { badges, active }) => {
                if !self.signed_in.get_untracked() {
                    self.signed_in.set(true);
                }
                if self.badges.get_untracked() != badges {
                    self.selected_medal.update(|selected| {
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
                if self.active.get_untracked() != active {
                    self.active.set(active);
                }
            }
            Err(Failure::Rejected) => {
                session::forget();
                self.sync.update_untracked(|s| s.stamp = None);
                self.clear_private();
            }
            Err(error) => self.error.set(Some(error)),
            Ok(_) => self.error.set(Some(Failure::Malformed)),
        }
    }

    pub fn open(self, task: api::Challenge, board: api::Board, offset: u32) {
        let Some(started) = self.begin(true) else {
            return;
        };
        let id = task.id;
        let entering = self.selected.get_untracked().is_none();
        let same_task = self
            .selected
            .with_untracked(|old| old.as_ref().is_some_and(|old| old.id == id));
        if self
            .selected
            .with_untracked(|old| old.as_ref() != Some(&task))
        {
            self.selected.set(Some(task));
        }
        self.board.set(board);
        self.offset.set(offset);
        if !same_task {
            self.rows.set(None);
            self.personal.set(None);
            self.hint.set(None);
            self.confirm_hint.set(None);
        }
        let stamp = session::marker();
        leptos::task::spawn_local(async move {
            self.load_detail(started, id, board, offset).await;
            if self.generation() != started {
                return;
            }
            if !same_task || self.personal.get_untracked().is_none() {
                self.load_personal(started, stamp.as_deref(), id).await;
            }
            self.finish(started);
            if entering {
                crate::islands::focus::after_render("#challenge-detail-title");
            }
        });
    }

    pub fn close(self, id: [u8; 16]) {
        let present = self.feed.with_untracked(|feed| {
            feed.as_ref()
                .is_some_and(|feed| feed.challenges.iter().any(|task| task.id == id))
        });
        self.sync.update_untracked(|s| s.return_to = Some(id));
        self.selected.set(None);
        self.confirm_hint.set(None);
        self.hint.set(None);
        crate::islands::focus::after_render(if present {
            format!("#challenge-open-{}", crate::data::id_hex(&id))
        } else {
            "#challenge-refresh".into()
        });
    }

    async fn load_detail(self, started: u64, id: [u8; 16], board: api::Board, offset: u32) {
        let result = request(api::Request::Detail {
            challenge: id,
            board,
            offset,
        })
        .await;
        if self.generation() != started {
            return;
        }
        match result {
            Ok(api::Response::Detail {
                challenge,
                standings,
                ..
            }) => {
                if self
                    .selected
                    .with_untracked(|old| old.as_ref() != Some(&challenge))
                {
                    self.selected.set(Some(challenge));
                }
                if self
                    .rows
                    .with_untracked(|old| old.as_ref() != Some(&standings))
                {
                    self.rows.set(Some(standings));
                }
            }
            Err(error) => self.error.set(Some(error)),
            Ok(_) => self.error.set(Some(Failure::Malformed)),
        }
    }

    async fn load_personal(self, started: u64, stamp: Option<&str>, id: [u8; 16]) {
        let Some(expected) = stamp else {
            return;
        };
        if !self.signed_in.get_untracked() {
            return;
        }
        let result = request(api::Request::Personal { challenge: id }).await;
        if self.generation() != started {
            return;
        }
        if session::marker().as_deref() != Some(expected) {
            self.clear_private();
            return;
        }
        match result {
            Ok(api::Response::Personal { progress }) => {
                if self
                    .personal
                    .with_untracked(|old| old.as_ref() != Some(&progress))
                {
                    self.personal.set(Some(progress));
                }
            }
            Err(Failure::Rejected) => {
                session::forget();
                self.sync.update_untracked(|s| s.stamp = None);
                self.clear_private();
            }
            Err(error) => self.error.set(Some(error)),
            Ok(_) => self.error.set(Some(Failure::Malformed)),
        }
    }

    pub fn change(self, operation: api::Request, id: [u8; 16]) {
        let Some(started) = self.begin(true) else {
            return;
        };
        self.confirm_hint.set(None);
        let stamp = session::marker();
        leptos::task::spawn_local(async move {
            let listing_changed = matches!(operation, api::Request::Visibility { .. });
            let result = request(operation).await;
            if self.generation() != started {
                return;
            }
            match result {
                Ok(api::Response::Hint { text }) => self.hint.set(Some(text)),
                Ok(api::Response::Updated) => {}
                Err(Failure::Rejected) => {
                    session::forget();
                    self.sync.update_untracked(|s| s.stamp = None);
                    self.clear_private();
                    self.finish(started);
                    return;
                }
                Err(error) => self.error.set(Some(error)),
                Ok(_) => self.error.set(Some(Failure::Malformed)),
            }
            self.load_personal(started, stamp.as_deref(), id).await;
            if self.generation() != started {
                return;
            }
            if listing_changed {
                self.load_detail(
                    started,
                    id,
                    self.board.get_untracked(),
                    self.offset.get_untracked(),
                )
                .await;
            }
            self.finish(started);
        });
    }
}
