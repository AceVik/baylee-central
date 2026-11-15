use super::{Failure, api, call, medals, session, wire};
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub(super) struct Hub {
    pub progress: RwSignal<Option<api::Progress>>,
    pub error: RwSignal<Option<Failure>>,
    pub busy: RwSignal<bool>,
    pub signed_in: RwSignal<bool>,
    pub selected: RwSignal<Option<medals::Preview>>,
    stamp: RwSignal<Option<String>>,
    generation: RwSignal<u64>,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            progress: RwSignal::new(None),
            error: RwSignal::new(None),
            busy: RwSignal::new(false),
            signed_in: RwSignal::new(false),
            selected: RwSignal::new(None),
            stamp: RwSignal::new(None),
            generation: RwSignal::new(0),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn session_changed(self) {
        self.generation
            .update(|value| *value = value.wrapping_add(1));
        self.progress.set(None);
        self.selected.set(None);
        self.stamp.set(None);
        self.busy.set(false);
        self.refresh();
    }

    pub fn refresh(self) {
        if self.busy.get_untracked() {
            return;
        }
        let stamp = session::marker();
        if self.stamp.get_untracked() != stamp {
            self.progress.set(None);
            self.selected.set(None);
            self.stamp.set(stamp.clone());
        }
        self.signed_in.set(stamp.is_some());
        self.error.set(None);
        if stamp.is_none() {
            return;
        }
        self.busy.set(true);
        let generation = self.generation.get_untracked();
        leptos::task::spawn_local(async move {
            let result = call(&wire::Request::TowerProgress {}).await;
            if self.generation.get_untracked() != generation {
                return;
            }
            if session::marker() != stamp {
                self.progress.set(None);
                self.selected.set(None);
                self.signed_in.set(false);
                self.busy.set(false);
                return;
            }
            match result {
                Ok(wire::Response::TowerProgress { progress }) if progress.validate().is_ok() => {
                    if self.progress.get_untracked() != Some(progress) {
                        self.progress.set(Some(progress));
                        self.selected.set(None);
                    }
                }
                Ok(_) => self.error.set(Some(Failure::Malformed)),
                Err(error) => {
                    self.error.set(Some(error));
                    if error == Failure::Rejected {
                        self.progress.set(None);
                        self.selected.set(None);
                        self.signed_in.set(false);
                    }
                }
            }
            self.busy.set(false);
        });
    }
}
