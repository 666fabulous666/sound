use synth_core::session::SessionState;

/// Manages undo/redo state snapshots for score editing.
pub struct UndoStack {
    undo_stack: Vec<SessionState>,
    redo_stack: Vec<SessionState>,
    max_entries: usize,
}

impl UndoStack {
    pub fn new(max_entries: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_entries,
        }
    }

    /// Push current state to undo stack before making a change.
    /// Clears the redo stack since we're branching history.
    pub fn push(&mut self, state: SessionState) {
        self.redo_stack.clear();
        self.undo_stack.push(state);

        // Enforce max entries limit
        if self.undo_stack.len() > self.max_entries {
            self.undo_stack.remove(0);
        }
    }

    /// Undo: push current state to redo, return previous state from undo stack.
    pub fn undo(&mut self, current: SessionState) -> Option<SessionState> {
        self.undo_stack.pop().map(|prev| {
            self.redo_stack.push(current);
            prev
        })
    }

    /// Redo: push current state to undo, return next state from redo stack.
    pub fn redo(&mut self, current: SessionState) -> Option<SessionState> {
        self.redo_stack.pop().map(|next| {
            self.undo_stack.push(current);
            next
        })
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear both stacks (for new score or load operations).
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
