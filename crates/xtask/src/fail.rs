//! The one error every task returns: a fixed sentence. It never carries a
//! value the task handled (a password, a file's contents, a tool's raw
//! output); a step that fails says which step, nothing more.

use std::fmt;

/// A failed task step, described by a fixed sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fail(String);

impl Fail {
    /// A failure described by `message`.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    /// The sentence.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Fail {}

/// The result of a task step.
pub type Result<T> = std::result::Result<T, Fail>;

/// Replaces whatever went wrong with a fixed sentence, dropping its details.
pub trait OrFail<T> {
    /// The value, or a [`Fail`] saying `message`.
    ///
    /// # Errors
    /// When `self` is an error or empty.
    fn or_fail(self, message: &str) -> Result<T>;
}

impl<T, E> OrFail<T> for std::result::Result<T, E> {
    fn or_fail(self, message: &str) -> Result<T> {
        self.map_err(|_| Fail::new(message))
    }
}

impl<T> OrFail<T> for Option<T> {
    fn or_fail(self, message: &str) -> Result<T> {
        self.ok_or_else(|| Fail::new(message))
    }
}

/// A task's outcome after its cleanup: the first failure, then any failure
/// of the cleanup, so neither hides the other.
pub(crate) fn after_cleanup(outcome: Result<()>, cleanup: Result<()>) -> Result<()> {
    match (outcome, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(failure), Ok(())) | (Ok(()), Err(failure)) => Err(failure),
        (Err(failure), Err(cleanup)) => Err(Fail::new(format!("{failure}; {cleanup}"))),
    }
}
