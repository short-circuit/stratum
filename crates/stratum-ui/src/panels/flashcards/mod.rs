//! Native Flashcards panel.
//!
//! [`FlashcardsView`] ports the shipped desktop `FlashcardsPanel`: the review
//! cursor, the question/answer reveal, the SM-2 rating row, the source/ease
//! caption and the completion/restart view. Provider calls stay behind the
//! [`crate::provider::PanelData`] seam; the state transitions live in [`model`]
//! and are unit-tested without a window.
//!
//! Rating qualities come from the shared [`stratum_app::flashcards`] table so
//! the native panel and the backend agree on the SM-2 grades. The
//! [`FLASHCARDS_I18N_KEYS`] list is the release gate for every string this
//! surface renders.

use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_app::flashcards::{rating_quality, FlashcardRating};
use stratum_i18n::t_global;

use crate::panels::PanelRequests;
use crate::provider::{FlashcardRow, ShellData};

pub mod model;
pub mod view;

/// Every translation key the Flashcards surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const FLASHCARDS_I18N_KEYS: &[&str] = &[
    "flashcards._ease",
    "flashcards._interval",
    "flashcards._next",
    "flashcards.answer",
    "flashcards.answer_label",
    "flashcards.card_of_count",
    "flashcards.click_to_reveal_answer",
    "flashcards.click_to_see_question",
    "flashcards.correct",
    "flashcards.error",
    "flashcards.flashcards",
    "flashcards.no_flashcards_found_create_blocks_with",
    "flashcards.properties_to_generate_cards",
    "flashcards.question",
    "flashcards.question_label",
    "flashcards.rating_blackout",
    "flashcards.rating_easy",
    "flashcards.rating_good",
    "flashcards.rating_hard",
    "flashcards.review_again_soon",
    "flashcards.reviewed_cards_one",
    "flashcards.reviewed_cards_other",
    "flashcards.session_complete",
    "flashcards.source",
    "flashcards.start_again",
];

/// Deterministic start state for the flashcards panel.
///
/// `STRATUM_FLASHCARD_REVEAL=1` uses this to reveal the first card's answer so
/// the capture harness can render the rating row without synthesising a click.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FlashcardsStart {
    /// Reveal the first card's answer after construction.
    pub reveal: bool,
}

/// The flashcards panel entity.
pub struct FlashcardsView {
    provider: Arc<dyn ShellData>,
    cards: Vec<FlashcardRow>,
    session: model::Session,
    review_error: Option<String>,
    pending_navigation: Option<String>,
    pending_toast: Option<String>,
}

impl FlashcardsView {
    /// Build the panel, load the cards and apply the requested start state.
    pub fn new(
        provider: Arc<dyn ShellData>,
        start: FlashcardsStart,
        cx: &mut Context<Self>,
    ) -> Self {
        let (cards, session) = match provider.flashcards() {
            Ok(cards) => {
                let session = model::Session::new(cards.len());
                (cards, session)
            }
            Err(error) => (Vec::new(), model::Session::failed(error)),
        };
        let mut view = Self {
            provider,
            cards,
            session,
            review_error: None,
            pending_navigation: None,
            pending_toast: None,
        };
        if start.reveal {
            view.reveal();
        }
        let _ = cx;
        view
    }

    /// The loaded cards.
    pub fn cards(&self) -> &[FlashcardRow] {
        &self.cards
    }

    /// The current session state.
    pub fn session(&self) -> &model::Session {
        &self.session
    }

    /// The current card, when a card is active.
    pub fn current_card(&self) -> Option<&FlashcardRow> {
        self.session
            .current_index()
            .and_then(|index| self.cards.get(index))
    }

    /// Whether the current card's answer is revealed.
    pub fn revealed(&self) -> bool {
        self.session.revealed()
    }

    /// The localized progress caption.
    pub fn progress_label(&self) -> String {
        self.session.progress_label()
    }

    /// The last review/provider error, if any.
    pub fn error(&self) -> Option<&str> {
        self.review_error.as_deref()
    }

    /// The localized rating feedback, if any.
    pub fn message(&self) -> Option<String> {
        self.session.message_key().map(t_global)
    }

    /// Reveal the current card's answer.
    pub fn reveal(&mut self) {
        self.session.reveal();
    }

    /// Flip the current card's reveal flag (shipped card click).
    pub fn toggle_reveal(&mut self, cx: &mut Context<Self>) {
        self.session.toggle_reveal();
        cx.notify();
    }

    /// Record a self-assessment for the current card through the provider.
    ///
    /// The session advances only after the provider accepts the review, so a
    /// backend failure leaves the card on screen and surfaces the error.
    pub fn rate(&mut self, rating: FlashcardRating, cx: &mut Context<Self>) {
        let Some(card) = self.current_card().cloned() else {
            return;
        };
        let quality = rating_quality(rating);
        match self
            .provider
            .review_card(&card.id, quality, &card.page_path)
        {
            Ok(()) => {
                self.review_error = None;
                self.session.rate(rating);
            }
            Err(error) => {
                self.review_error = Some(format!("{} {error}", t_global("flashcards.error")));
            }
        }
        cx.notify();
    }

    /// Restart the session at the first card.
    pub fn restart(&mut self, cx: &mut Context<Self>) {
        self.review_error = None;
        self.session.restart();
        cx.notify();
    }
}

impl PanelRequests for FlashcardsView {
    fn pending_navigation(&self) -> Option<&str> {
        self.pending_navigation.as_deref()
    }

    fn clear_pending_navigation(&mut self) {
        self.pending_navigation = None;
    }

    fn pending_toast(&self) -> Option<&str> {
        self.pending_toast.as_deref()
    }

    fn take_pending_toast(&mut self) -> Option<String> {
        self.pending_toast.take()
    }
}

impl Render for FlashcardsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        view::render(self, cx)
    }
}
