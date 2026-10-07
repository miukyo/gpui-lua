use super::change::Change;
use super::cursor::CursorSelection;

const MAX_UNDO_TRANSACTIONS: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditIntent {
    Typing,
    Backspace,
    DeleteForward,
    Atomic,
}

#[derive(Debug)]
struct UndoTransaction {
    intent: EditIntent,
    changes: Vec<Change>,
    selections_before: Option<Vec<CursorSelection>>,
    selections_after: Option<Vec<CursorSelection>>,
}

#[derive(Debug)]
struct PendingTransaction {
    intent: EditIntent,
    changes: Vec<Change>,
    selections_before: Option<Vec<CursorSelection>>,
    selections_after: Option<Vec<CursorSelection>>,
}

pub struct Replay {
    pub changes: Vec<Change>,
    pub selections: Option<Vec<CursorSelection>>,
}

#[derive(Debug, Default)]
pub struct UndoManager {
    undo_transactions: Vec<UndoTransaction>,
    redo_transactions: Vec<UndoTransaction>,
    ignoring: bool,
    transaction_depth: usize,
    pending: Option<PendingTransaction>,
    pending_intent: Option<EditIntent>,
    coalescing_boundary: bool,
}

impl UndoManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn take_pending_intent(&mut self) -> Option<EditIntent> {
        self.pending_intent.take()
    }

    pub fn set_pending_intent(&mut self, intent: EditIntent) {
        self.pending_intent = Some(intent);
    }

    pub fn record_transaction(&mut self, change: Change, intent: EditIntent) -> bool {
        if self.ignoring {
            return false;
        }
        if change.old_range == change.new_range && change.old_text == change.new_text {
            self.break_transaction_coalescing();
            return false;
        }

        match self.pending.as_mut() {
            Some(pending) => pending.changes.push(change),
            None => self.push_batch(vec![change], intent),
        }
        true
    }

    pub fn begin_transaction(&mut self) {
        self.begin_transaction_with(EditIntent::Atomic);
    }

    pub fn begin_transaction_with(&mut self, intent: EditIntent) {
        if self.ignoring {
            return;
        }
        self.transaction_depth += 1;
        if self.transaction_depth == 1 {
            self.pending = Some(PendingTransaction {
                intent,
                changes: Vec::new(),
                selections_before: None,
                selections_after: None,
            });
        }
    }

    pub fn commit_transaction(&mut self) {
        if self.ignoring || self.transaction_depth == 0 {
            return;
        }
        self.transaction_depth -= 1;
        if self.transaction_depth == 0 {
            if let Some(pending) = self.pending.take() {
                if !pending.changes.is_empty() {
                    let tx = UndoTransaction {
                        intent: pending.intent,
                        changes: pending.changes,
                        selections_before: pending.selections_before,
                        selections_after: pending.selections_after,
                    };
                    self.push_transaction(tx);
                }
            }
        }
    }

    fn push_batch(&mut self, changes: Vec<Change>, intent: EditIntent) {
        self.push_transaction(UndoTransaction {
            intent,
            changes,
            selections_before: None,
            selections_after: None,
        });
    }

    fn push_transaction(&mut self, mut tx: UndoTransaction) {
        self.redo_transactions.clear();

        // Try coalescing with previous transaction if compatible
        if !self.coalescing_boundary && !self.undo_transactions.is_empty() {
            let last = self.undo_transactions.last_mut().unwrap();
            if last.intent == tx.intent && tx.intent == EditIntent::Typing && tx.changes.len() == 1 {
                let change = tx.changes.remove(0);
                if let Some(last_change) = last.changes.last_mut() {
                    if last_change.new_range.end == change.old_range.start {
                        last_change.new_range.end = change.new_range.end;
                        last_change.new_text.push_str(&change.new_text);
                        last.selections_after = tx.selections_after;
                        return;
                    }
                }
            }
        }

        self.coalescing_boundary = false;
        if self.undo_transactions.len() >= MAX_UNDO_TRANSACTIONS {
            self.undo_transactions.remove(0);
        }
        self.undo_transactions.push(tx);
    }

    pub fn break_transaction_coalescing(&mut self) {
        self.coalescing_boundary = true;
    }

    pub fn undo(&mut self) -> Option<Replay> {
        let tx = self.undo_transactions.pop()?;
        let mut inverted_changes = Vec::new();
        for change in tx.changes.iter().rev() {
            inverted_changes.push(Change {
                old_range: change.new_range,
                old_text: change.new_text.clone(),
                new_range: change.old_range,
                new_text: change.old_text.clone(),
            });
        }

        let selections_before = tx.selections_before.clone();
        self.redo_transactions.push(tx);

        Some(Replay {
            changes: inverted_changes,
            selections: selections_before,
        })
    }

    pub fn redo(&mut self) -> Option<Replay> {
        let tx = self.redo_transactions.pop()?;
        let changes = tx.changes.clone();
        let selections_after = tx.selections_after.clone();
        self.undo_transactions.push(tx);

        Some(Replay {
            changes,
            selections: selections_after,
        })
    }
}
