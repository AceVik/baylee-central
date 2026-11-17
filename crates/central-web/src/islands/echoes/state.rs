use super::{Failure, api, call, medals, motif_of, session, wire};
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub(super) struct Hub {
    pub progress: RwSignal<Option<api::Progress>>,
    pub error: RwSignal<Option<Failure>>,
    pub busy: RwSignal<bool>,
    pub loading: RwSignal<bool>,
    pub signed_in: RwSignal<bool>,
    pub selected: RwSignal<Option<medals::Preview>>,
    in_flight: RwSignal<bool>,
    stamp: RwSignal<Option<String>>,
    generation: RwSignal<u64>,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            progress: RwSignal::new(None),
            error: RwSignal::new(None),
            busy: RwSignal::new(true),
            loading: RwSignal::new(false),
            signed_in: RwSignal::new(false),
            selected: RwSignal::new(None),
            in_flight: RwSignal::new(false),
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
                .update(|value| *value = value.wrapping_add(1));
        } else if self.busy.get_untracked() || self.in_flight.get_untracked() {
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
        if interactive {
            self.busy.set(true);
            self.loading.set(true);
        }
        self.in_flight.set(true);
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
                self.loading.set(false);
                self.in_flight.set(false);
                return;
            }
            match result {
                Ok(wire::Response::TowerProgress { progress }) if progress.validate().is_ok() => {
                    if self.progress.get_untracked() != Some(progress) {
                        self.selected.update(|selected| {
                            if let Some(preview) = selected
                                && let Some(updated) = progress
                                    .badges()
                                    .into_iter()
                                    .find(|b| motif_of(b.kind) == preview.motif)
                            {
                                preview.count = u32::try_from(updated.count).unwrap_or(u32::MAX);
                            }
                        });
                        self.progress.set(Some(progress));
                    }
                }
                Ok(_) => self.error.set(Some(Failure::Malformed)),
                Err(error) => {
                    self.error.set(Some(error));
                    if error == Failure::Rejected {
                        session::forget();
                        self.stamp.set(None);
                        self.progress.set(None);
                        self.selected.set(None);
                        self.signed_in.set(false);
                    }
                }
            }
            self.busy.set(false);
            self.loading.set(false);
            self.in_flight.set(false);
        });
    }
}
