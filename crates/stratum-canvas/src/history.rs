//! Bounded undo/redo delta journal for canvas mutations.
//!
//! Every mutation in [`crate::tools::CanvasState`] records a [`Delta`]. The
//! journal keeps at most [`HISTORY_LIMIT`] entries; older entries fall off the
//! bottom. A new mutation clears the redo stack, matching Excalidraw's history
//! semantics. Undo reverses one delta and moves it to the redo stack; redo
//! reapplies it.
//!
//! The journal stores full element snapshots for updates so unknown keys are
//! restored exactly when an edit is undone.

use pkm_core::{PkmError, PkmResult};

use crate::document::BoardDocument;
use crate::element::Element;

/// Maximum number of undo entries retained.
pub const HISTORY_LIMIT: usize = 100;

/// One reversible mutation.
#[derive(Debug, Clone)]
pub enum Delta {
    /// An element was appended at `index`.
    Insert {
        /// Z-order index the element was inserted at.
        index: usize,
        /// Inserted element snapshot.
        element: Element,
    },
    /// An element entry was removed from `index`.
    Remove {
        /// Z-order index the element occupied.
        index: usize,
        /// Removed element snapshot.
        element: Element,
    },
    /// An element was modified from `before` to `after`.
    Update {
        /// Element id.
        id: String,
        /// State before the mutation.
        before: Element,
        /// State after the mutation.
        after: Element,
    },
    /// An element was moved in z-order.
    Reorder {
        /// Element id.
        id: String,
        /// Previous z-order index.
        from: usize,
        /// New z-order index.
        to: usize,
    },
    /// Group membership changed for a set of elements.
    Group {
        /// Element ids.
        ids: Vec<String>,
        /// Group ids before the change, aligned with `ids`.
        before: Vec<Vec<String>>,
        /// Group ids after the change, aligned with `ids`.
        after: Vec<Vec<String>>,
    },
}

/// Undo/redo journal.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Delta>,
    redo: Vec<Delta>,
}

impl History {
    /// Create an empty journal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a mutation and clear the redo stack.
    pub fn record(&mut self, delta: Delta) {
        self.redo.clear();
        self.undo.push(delta);
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
    }

    /// Drop the redo stack without recording a mutation (used when an external
    /// caller mutates the document directly).
    pub fn clear_redo(&mut self) {
        self.redo.clear();
    }

    /// Number of undo entries.
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Number of redo entries.
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Whether an undo entry is available.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether a redo entry is available.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Reverse the most recent mutation.
    ///
    /// Returns whether anything was undone.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when a delta cannot be applied (for example
    /// an update for an element that no longer exists).
    pub fn undo(&mut self, doc: &mut BoardDocument) -> PkmResult<bool> {
        let Some(delta) = self.undo.pop() else {
            return Ok(false);
        };
        revert(doc, &delta)?;
        self.redo.push(delta);
        Ok(true)
    }

    /// Reapply the most recently undone mutation.
    ///
    /// Returns whether anything was redone.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when a delta cannot be applied.
    pub fn redo(&mut self, doc: &mut BoardDocument) -> PkmResult<bool> {
        let Some(delta) = self.redo.pop() else {
            return Ok(false);
        };
        apply(doc, &delta)?;
        self.undo.push(delta);
        Ok(true)
    }
}

/// Apply a delta (redo direction).
fn apply(doc: &mut BoardDocument, delta: &Delta) -> PkmResult<()> {
    match delta {
        Delta::Insert { index, element } => {
            let mut elements = doc.elements_snapshot();
            if elements
                .iter()
                .any(|existing| existing.id() == element.id())
            {
                return Err(PkmError::Canvas(format!(
                    "cannot insert duplicate element {}",
                    element.id()
                )));
            }
            elements.insert((*index).min(elements.len()), element.clone());
            doc.replace_elements(elements);
        }
        Delta::Remove { element, .. } => {
            let mut elements = doc.elements_snapshot();
            elements.retain(|existing| existing.id() != element.id());
            doc.replace_elements(elements);
        }
        Delta::Update { id, after, .. } => replace_element(doc, id, after)?,
        Delta::Reorder { id, to, .. } => move_element(doc, id, *to)?,
        Delta::Group { ids, after, .. } => set_groups(doc, ids, after)?,
    }
    doc.mark_dirty();
    Ok(())
}

/// Revert a delta (undo direction).
fn revert(doc: &mut BoardDocument, delta: &Delta) -> PkmResult<()> {
    match delta {
        Delta::Insert { element, .. } => {
            let mut elements = doc.elements_snapshot();
            elements.retain(|existing| existing.id() != element.id());
            doc.replace_elements(elements);
        }
        Delta::Remove { index, element } => {
            let mut elements = doc.elements_snapshot();
            if elements
                .iter()
                .any(|existing| existing.id() == element.id())
            {
                return Err(PkmError::Canvas(format!(
                    "cannot restore duplicate element {}",
                    element.id()
                )));
            }
            elements.insert((*index).min(elements.len()), element.clone());
            doc.replace_elements(elements);
        }
        Delta::Update { id, before, .. } => replace_element(doc, id, before)?,
        Delta::Reorder { id, from, .. } => move_element(doc, id, *from)?,
        Delta::Group { ids, before, .. } => set_groups(doc, ids, before)?,
    }
    doc.mark_dirty();
    Ok(())
}

/// Replace one element in place.
fn replace_element(doc: &mut BoardDocument, id: &str, replacement: &Element) -> PkmResult<()> {
    let mut elements = doc.elements_snapshot();
    let Some(index) = elements.iter().position(|element| element.id() == id) else {
        return Err(PkmError::Canvas(format!("element {id} no longer exists")));
    };
    elements[index] = replacement.clone();
    doc.replace_elements(elements);
    Ok(())
}

/// Move an element to a z-order index.
fn move_element(doc: &mut BoardDocument, id: &str, target: usize) -> PkmResult<()> {
    let mut elements = doc.elements_snapshot();
    let Some(index) = elements.iter().position(|element| element.id() == id) else {
        return Err(PkmError::Canvas(format!("element {id} no longer exists")));
    };
    let element = elements.remove(index);
    let target = target.min(elements.len());
    elements.insert(target, element);
    doc.replace_elements(elements);
    Ok(())
}

/// Set group ids for a set of elements.
fn set_groups(doc: &mut BoardDocument, ids: &[String], groups: &[Vec<String>]) -> PkmResult<()> {
    let mut elements = doc.elements_snapshot();
    for (id, group) in ids.iter().zip(groups.iter()) {
        let Some(element) = elements.iter_mut().find(|element| element.id() == id) else {
            return Err(PkmError::Canvas(format!("element {id} no longer exists")));
        };
        element.set_group_ids(group);
    }
    doc.replace_elements(elements);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element_kind::ElementKind;

    #[test]
    fn undo_insert_removes_element() {
        let mut doc = BoardDocument::empty();
        let element = Element::new(ElementKind::Rectangle, "r1", 0.0, 0.0, 10.0, 10.0);
        doc.add_element(element.clone()).unwrap();
        let mut history = History::new();
        history.record(Delta::Insert {
            index: 0,
            element: element.clone(),
        });
        assert!(history.undo(&mut doc).unwrap());
        assert!(doc.element("r1").is_none());
        assert!(history.redo(&mut doc).unwrap());
        assert!(doc.element("r1").is_some());
    }

    #[test]
    fn new_mutation_clears_redo() {
        let mut doc = BoardDocument::empty();
        let element = Element::new(ElementKind::Rectangle, "r1", 0.0, 0.0, 10.0, 10.0);
        doc.add_element(element.clone()).unwrap();
        let mut history = History::new();
        history.record(Delta::Insert {
            index: 0,
            element: element.clone(),
        });
        history.undo(&mut doc).unwrap();
        assert!(history.can_redo());
        history.record(Delta::Insert {
            index: 0,
            element: Element::new(ElementKind::Ellipse, "e1", 0.0, 0.0, 10.0, 10.0),
        });
        assert!(!history.can_redo());
    }

    #[test]
    fn journal_is_bounded() {
        let mut history = History::new();
        for index in 0..(HISTORY_LIMIT + 20) {
            history.record(Delta::Insert {
                index,
                element: Element::new(
                    ElementKind::Rectangle,
                    &format!("r{index}"),
                    0.0,
                    0.0,
                    1.0,
                    1.0,
                ),
            });
        }
        assert_eq!(history.undo_len(), HISTORY_LIMIT);
    }
}
