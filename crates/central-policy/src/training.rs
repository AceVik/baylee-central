//! Training preference rules, deliberately separate from export authorization.
//!
//! Human replay export remains disabled here. One-game intent is not proof that
//! the other participants agreed, that the data is anonymous, or that the
//! processing has a lawful basis.

/// Durable account preference. No choice is not consent.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Preference {
    /// No affirmative choice has been recorded.
    #[default]
    Unset,
    /// A user deliberately enabled future automatic contributions.
    Enabled,
    /// A user deliberately disabled automatic contributions.
    Disabled,
}

/// A monotonically revisioned preference.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PreferenceState {
    preference: Preference,
    revision: u64,
}

/// Concurrency or lifetime failure; the original state is preserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreferenceError {
    /// Another device has changed the preference since it was read.
    StaleRevision,
    /// Revisions must never wrap and make old writes appear current.
    RevisionExhausted,
}

/// Intent for exactly one game, not an account-wide setting.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GameChoice {
    /// Default result-screen state: nothing selected.
    #[default]
    NotSelected,
    /// The user deliberately selected contribution of this game.
    Selected,
}

impl PreferenceState {
    /// Restore trusted persisted state. This does not validate a caller's identity.
    #[must_use]
    pub const fn from_persisted(preference: Preference, revision: u64) -> Self {
        Self {
            preference,
            revision,
        }
    }

    /// Current account preference.
    #[must_use]
    pub const fn preference(self) -> Preference {
        self.preference
    }

    /// Revision for conditional persistence.
    #[must_use]
    pub const fn revision(self) -> u64 {
        self.revision
    }

    /// Calculate an explicit update. The caller must persist it atomically using
    /// a database compare-and-swap on the same revision, under account ownership.
    ///
    /// # Errors
    ///
    /// Rejects stale writes and revision overflow without changing this value.
    pub fn changed(self, expected_revision: u64, enable: bool) -> Result<Self, PreferenceError> {
        if self.revision != expected_revision {
            return Err(PreferenceError::StaleRevision);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(PreferenceError::RevisionExhausted)?;
        Ok(Self {
            preference: if enable {
                Preference::Enabled
            } else {
                Preference::Disabled
            },
            revision,
        })
    }

    /// Whether the user expressed contribution intent for a game.
    ///
    /// The authoritative latest preference must be read at intake. Persisting a
    /// one-game choice also needs a bound account/game/revision and authenticated
    /// operation; this function is NOT an export authorization decision.
    #[must_use]
    pub const fn contribution_intent(self, game: GameChoice) -> bool {
        matches!(self.preference, Preference::Enabled) || matches!(game, GameChoice::Selected)
    }
}

/// Production human replay export is unconditionally disabled in this foundation.
/// There is intentionally no environment variable or public flag to enable it.
#[must_use]
pub const fn human_export_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_choice_is_not_consent() {
        let state = PreferenceState::default();
        assert!(!state.contribution_intent(GameChoice::default()));
        assert!(!human_export_available());
    }

    #[test]
    fn all_preference_and_game_choice_combinations() {
        for preference in [Preference::Unset, Preference::Enabled, Preference::Disabled] {
            for game in [GameChoice::NotSelected, GameChoice::Selected] {
                let state = PreferenceState::from_persisted(preference, 42);
                let result = state.contribution_intent(game);
                let expected = preference == Preference::Enabled || game == GameChoice::Selected;
                assert_eq!(result, expected);
                assert_eq!(
                    state.preference(),
                    preference,
                    "one-game intent cannot change global state"
                );
                assert_eq!(state.revision(), 42);
                assert!(
                    !human_export_available(),
                    "intent must not bypass the release gate"
                );
            }
        }
    }

    #[test]
    fn stale_device_cannot_reenable_after_opt_out() {
        let initial = PreferenceState::default();
        let enabled = initial.changed(0, true).unwrap();
        let disabled = enabled.changed(1, false).unwrap();
        assert_eq!(
            disabled.changed(1, true),
            Err(PreferenceError::StaleRevision)
        );
        assert!(!disabled.contribution_intent(GameChoice::NotSelected));
        assert!(disabled.contribution_intent(GameChoice::Selected));
        assert_eq!(disabled.preference(), Preference::Disabled);
    }

    #[test]
    fn revision_overflow_fails_closed() {
        for preference in [Preference::Unset, Preference::Enabled, Preference::Disabled] {
            let state = PreferenceState::from_persisted(preference, u64::MAX);
            assert_eq!(
                state.changed(u64::MAX, true),
                Err(PreferenceError::RevisionExhausted)
            );
            assert_eq!(
                state.changed(u64::MAX, false),
                Err(PreferenceError::RevisionExhausted)
            );
            assert_eq!(state.preference(), preference);
        }
    }

    #[test]
    fn repeated_same_choice_still_invalidates_older_operations() {
        let state = PreferenceState::default().changed(0, false).unwrap();
        let updated = state.changed(1, false).unwrap();
        assert_eq!(updated.revision(), 2);
        assert_eq!(updated.preference(), Preference::Disabled);
    }
}
