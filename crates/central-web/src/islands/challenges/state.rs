use super::{ChallengeFeed, Failure, api, request, session};
use leptos::prelude::*;

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
    pub signed_in: RwSignal<bool>,
    pub busy: RwSignal<bool>,
    pub error: RwSignal<Option<Failure>>,
    pub hint: RwSignal<Option<api::Words>>,
    pub confirm_hint: RwSignal<Option<u8>>,
    pub page: RwSignal<Option<i64>>,
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
            signed_in: RwSignal::new(false),
            // Server-rendered controls cannot act until hydration attaches their handlers.
            busy: RwSignal::new(true),
            error: RwSignal::new(None),
            hint: RwSignal::new(None),
            confirm_hint: RwSignal::new(None),
            page: RwSignal::new(None),
        }
    }

    fn start(self) -> bool {
        if self.busy.get_untracked() {
            return false;
        }
        self.busy.set(true);
        self.error.set(None);
        true
    }

    pub fn refresh(self, before: Option<i64>) {
        if !self.start() {
            return;
        }
        self.page.set(before);
        leptos::task::spawn_local(async move {
            match request(api::Request::Catalogue { before }).await {
                Ok(api::Response::Catalogue {
                    now,
                    challenges,
                    before,
                }) => self.feed.set(Some(ChallengeFeed {
                    now,
                    challenges,
                    before,
                })),
                Ok(_) => self.error.set(Some(Failure::Malformed)),
                Err(error) => self.error.set(Some(error)),
            }
            self.load_progress().await;
            if let Some(task) = self.selected.get_untracked() {
                self.load_detail(
                    task.id,
                    self.board.get_untracked(),
                    self.offset.get_untracked(),
                )
                .await;
                self.load_personal(task.id).await;
            }
            self.busy.set(false);
        });
    }

    async fn load_progress(self) {
        if session::remembered().is_none() {
            self.signed_in.set(false);
            self.badges.set(vec![]);
            self.active.set(vec![]);
            self.personal.set(None);
            self.hint.set(None);
            self.confirm_hint.set(None);
            return;
        }
        match request(api::Request::Progress).await {
            Ok(api::Response::Progress { badges, active }) => {
                self.signed_in.set(true);
                self.badges.set(badges);
                self.active.set(active);
            }
            Err(Failure::Rejected) => {
                session::forget();
                self.signed_in.set(false);
                self.personal.set(None);
                self.badges.set(vec![]);
                self.active.set(vec![]);
                self.hint.set(None);
                self.confirm_hint.set(None);
            }
            Err(error) => self.error.set(Some(error)),
            Ok(_) => self.error.set(Some(Failure::Malformed)),
        }
    }

    pub fn open(self, task: api::Challenge, board: api::Board, offset: u32) {
        if !self.start() {
            return;
        }
        let id = task.id;
        self.selected.set(Some(task));
        self.board.set(board);
        self.offset.set(offset);
        self.rows.set(None);
        self.personal.set(None);
        self.hint.set(None);
        self.confirm_hint.set(None);
        leptos::task::spawn_local(async move {
            self.load_detail(id, board, offset).await;
            self.load_personal(id).await;
            self.busy.set(false);
        });
    }

    async fn load_detail(self, id: [u8; 16], board: api::Board, offset: u32) {
        match request(api::Request::Detail {
            challenge: id,
            board,
            offset,
        })
        .await
        {
            Ok(api::Response::Detail {
                challenge,
                standings,
                ..
            }) => {
                self.selected.set(Some(challenge));
                self.rows.set(Some(standings));
            }
            Err(error) => self.error.set(Some(error)),
            Ok(_) => self.error.set(Some(Failure::Malformed)),
        }
    }

    async fn load_personal(self, id: [u8; 16]) {
        if !self.signed_in.get_untracked() {
            return;
        }
        match request(api::Request::Personal { challenge: id }).await {
            Ok(api::Response::Personal { progress }) => self.personal.set(Some(progress)),
            Err(Failure::Rejected) => {
                self.signed_in.set(false);
                self.personal.set(None);
            }
            Err(error) => self.error.set(Some(error)),
            Ok(_) => self.error.set(Some(Failure::Malformed)),
        }
    }

    pub fn change(self, operation: api::Request, id: [u8; 16]) {
        if !self.start() {
            return;
        }
        self.confirm_hint.set(None);
        leptos::task::spawn_local(async move {
            let listing_changed = matches!(operation, api::Request::Visibility { .. });
            match request(operation).await {
                Ok(api::Response::Hint { text }) => self.hint.set(Some(text)),
                Ok(api::Response::Updated) => {}
                Err(error) => self.error.set(Some(error)),
                Ok(_) => self.error.set(Some(Failure::Malformed)),
            }
            self.load_personal(id).await;
            if listing_changed {
                self.load_detail(id, self.board.get_untracked(), self.offset.get_untracked())
                    .await;
            }
            self.busy.set(false);
        });
    }
}
