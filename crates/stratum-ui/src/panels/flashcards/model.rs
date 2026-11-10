//! Pure flashcard-session state machine.
//!
//! The shipped `FlashcardsPanel` keeps the review cursor, the reveal flag and a
//! transient message in component state. That logic lives here so the reveal,
//! rating, completion and restart transitions can be unit-tested without a
//! window. Rating qualities come from the shared
//! [`stratum_app::flashcards`] table; the message is stored as a catalog key so
//! this module stays locale-independent and the panel resolves it at render
//! time.

use stratum_app::flashcards::{is_correct, rating_quality, FlashcardRating};
use stratum_i18n::tr_global;

/// Catalog key shown after a correct self-assessment.
pub const CORRECT_MESSAGE_KEY: &str = "flashcards.correct";
/// Catalog key shown after an incorrect self-assessment.
pub const AGAIN_MESSAGE_KEY: &str = "flashcards.review_again_soon";

/// One flashcard session state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionState {
    /// Cards are still loading.
    Loading,
    /// The vault has no flashcards.
    Empty,
    /// A card is ready for review at `index`.
    Card {
        /// Zero-based index into the card list.
        index: usize,
    },
    /// Every card has been reviewed.
    Complete,
    /// Loading failed with a stable error message.
    Failed(String),
}

/// A review session over `total` cards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    /// Current session state.
    pub state: SessionState,
    /// Number of cards in the session.
    pub total: usize,
    /// Whether the current card's answer is revealed.
    pub revealed: bool,
    /// Catalog key for the last rating feedback, if any.
    pub message_key: Option<&'static str>,
}

impl Session {
    /// Start a loading session.
    pub fn loading() -> Self {
        Self {
            state: SessionState::Loading,
            total: 0,
            revealed: false,
            message_key: None,
        }
    }

    /// Start a session over `total` cards.
    pub fn new(total: usize) -> Self {
        Self {
            state: if total == 0 {
                SessionState::Empty
            } else {
                SessionState::Card { index: 0 }
            },
            total,
            revealed: false,
            message_key: None,
        }
    }

    /// Start a failed session.
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            state: SessionState::Failed(error.into()),
            total: 0,
            revealed: false,
            message_key: None,
        }
    }

    /// The current state.
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// Whether the current card's answer is visible.
    pub fn revealed(&self) -> bool {
        self.revealed
    }

    /// The current card index, when a card is active.
    pub fn current_index(&self) -> Option<usize> {
        match self.state {
            SessionState::Card { index } => Some(index),
            _ => None,
        }
    }

    /// The catalog key for the last rating feedback, if any.
    pub fn message_key(&self) -> Option<&'static str> {
        self.message_key
    }

    /// Reveal the current card's answer.
    ///
    /// A no-op unless a card is active. Revealing also clears the previous
    /// rating feedback, matching the shipped panel where the transient message
    /// disappears before the next answer is shown.
    pub fn reveal(&mut self) {
        if matches!(self.state, SessionState::Card { .. }) {
            self.revealed = true;
            self.message_key = None;
        }
    }

    /// Flip the reveal flag for the active card (shipped card click).
    ///
    /// A no-op unless a card is active. Turning the answer on clears the
    /// previous rating feedback.
    pub fn toggle_reveal(&mut self) {
        if matches!(self.state, SessionState::Card { .. }) {
            self.revealed = !self.revealed;
            if self.revealed {
                self.message_key = None;
            }
        }
    }

    /// Apply a self-assessment to the active card.
    ///
    /// Ratings are ignored until the answer is revealed, matching the shipped
    /// panel where the rating row only exists after the reveal. A rating sets
    /// the feedback key, hides the answer and advances to the next card, or
    /// completes the session after the last card.
    pub fn rate(&mut self, rating: FlashcardRating) {
        let SessionState::Card { index } = self.state else {
            return;
        };
        if !self.revealed {
            return;
        }
        let quality = rating_quality(rating);
        self.message_key = Some(if is_correct(quality) {
            CORRECT_MESSAGE_KEY
        } else {
            AGAIN_MESSAGE_KEY
        });
        self.revealed = false;
        let next = index + 1;
        self.state = if next >= self.total {
            SessionState::Complete
        } else {
            SessionState::Card { index: next }
        };
    }

    /// Restart the session at the first card.
    ///
    /// A session with no cards stays [`SessionState::Empty`].
    pub fn restart(&mut self) {
        self.revealed = false;
        self.message_key = None;
        self.state = if self.total == 0 {
            SessionState::Empty
        } else {
            SessionState::Card { index: 0 }
        };
    }

    /// The localized progress caption (`Card N of M`).
    pub fn progress_label(&self) -> String {
        let current = match self.state {
            SessionState::Card { index } => index + 1,
            SessionState::Complete => self.total,
            _ => 0,
        };
        let current = current.to_string();
        let total = self.total.to_string();
        tr_global(
            "flashcards.card_of_count",
            &[("current", current.as_str()), ("total", total.as_str())],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_starts_on_the_first_card() {
        let session = Session::new(2);
        assert_eq!(session.state(), &SessionState::Card { index: 0 });
        assert!(!session.revealed());
        assert_eq!(session.progress_label(), "Card 1 of 2");
    }

    #[test]
    fn empty_session_has_no_cards() {
        let mut session = Session::new(0);
        assert_eq!(session.state(), &SessionState::Empty);
        session.restart();
        assert_eq!(session.state(), &SessionState::Empty);
    }

    #[test]
    fn reveal_only_affects_active_cards() {
        let mut session = Session::new(1);
        session.reveal();
        assert!(session.revealed());
        session.toggle_reveal();
        assert!(!session.revealed());

        let mut empty = Session::new(0);
        empty.reveal();
        assert!(!empty.revealed());
        empty.toggle_reveal();
        assert!(!empty.revealed());
    }

    #[test]
    fn rating_requires_a_revealed_answer() {
        let mut session = Session::new(2);
        session.rate(FlashcardRating::Good);
        assert_eq!(session.state(), &SessionState::Card { index: 0 });
        assert_eq!(session.message_key(), None);
    }

    #[test]
    fn correct_rating_advances_and_sets_the_feedback_key() {
        let mut session = Session::new(2);
        session.reveal();
        session.rate(FlashcardRating::Good);
        assert_eq!(session.state(), &SessionState::Card { index: 1 });
        assert!(!session.revealed());
        assert_eq!(session.message_key(), Some(CORRECT_MESSAGE_KEY));
        assert_eq!(session.progress_label(), "Card 2 of 2");
    }

    #[test]
    fn incorrect_rating_advances_and_sets_the_retry_key() {
        let mut session = Session::new(2);
        session.reveal();
        session.rate(FlashcardRating::Hard);
        assert_eq!(session.state(), &SessionState::Card { index: 1 });
        assert_eq!(session.message_key(), Some(AGAIN_MESSAGE_KEY));
        assert!(!is_correct(rating_quality(FlashcardRating::Hard)));
    }

    #[test]
    fn rating_the_last_card_completes_the_session() {
        let mut session = Session::new(2);
        session.reveal();
        session.rate(FlashcardRating::Easy);
        session.reveal();
        session.rate(FlashcardRating::Blackout);
        assert_eq!(session.state(), &SessionState::Complete);
        assert!(!session.revealed());
        assert_eq!(session.progress_label(), "Card 2 of 2");
    }

    #[test]
    fn rating_a_complete_session_is_a_no_op() {
        let mut session = Session::new(1);
        session.reveal();
        session.rate(FlashcardRating::Good);
        let before = session.clone();
        session.rate(FlashcardRating::Easy);
        assert_eq!(session, before);
    }

    #[test]
    fn restart_returns_to_the_first_card_and_clears_feedback() {
        let mut session = Session::new(1);
        session.reveal();
        session.rate(FlashcardRating::Good);
        assert_eq!(session.state(), &SessionState::Complete);
        session.restart();
        assert_eq!(session.state(), &SessionState::Card { index: 0 });
        assert!(!session.revealed());
        assert_eq!(session.message_key(), None);
    }

    #[test]
    fn failed_state_keeps_the_error() {
        let session = Session::failed("boom");
        assert_eq!(session.state(), &SessionState::Failed("boom".to_string()));
        assert_eq!(session.progress_label(), "Card 0 of 0");
    }

    #[test]
    fn loading_state_has_no_progress() {
        let session = Session::loading();
        assert_eq!(session.state(), &SessionState::Loading);
        assert_eq!(session.progress_label(), "Card 0 of 0");
    }
}
