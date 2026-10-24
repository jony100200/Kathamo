//! Deterministic Redux Store and Transactional Undo/Redo Manager.
//!
//! Provides unidirectional data flow, pure state reducers, and bounded-capacity
//! snapshot history with zero secondary thrashing.

use std::collections::VecDeque;

/// Type alias for pure state reducers.
pub type Reducer<S, A> = fn(&mut S, &A);

/// Deterministic Redux store for studio applications.
pub struct Store<S, A> {
    state: S,
    reducer: Reducer<S, A>,
    listeners: Vec<Box<dyn Fn(&S)>>,
}

impl<S: Clone, A> Store<S, A> {
    pub fn new(initial_state: S, reducer: Reducer<S, A>) -> Self {
        Self {
            state: initial_state,
            reducer,
            listeners: Vec::new(),
        }
    }

    /// Access current immutable state snapshot.
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Dispatches an action, mutating state deterministically via reducer,
    /// and synchronously notifying all registered subscribers.
    pub fn dispatch(&mut self, action: A) {
        (self.reducer)(&mut self.state, &action);
        for listener in &self.listeners {
            listener(&self.state);
        }
    }

    /// Subscribes a listener to state mutation events.
    pub fn subscribe<F>(&mut self, listener: F)
    where
        F: Fn(&S) + 'static,
    {
        self.listeners.push(Box::new(listener));
    }
}

/// Transactional Undo/Redo manager maintaining bounded state snapshot histories.
#[derive(Debug, Clone)]
pub struct UndoManager<S> {
    undo_stack: VecDeque<S>,
    redo_stack: VecDeque<S>,
    max_history: usize,
}

impl<S: Clone> UndoManager<S> {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_history: max_history.max(1),
        }
    }

    /// Records a state snapshot before mutation. Clears the redo stack.
    pub fn record_snapshot(&mut self, snapshot: S) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(snapshot);
        self.redo_stack.clear();
    }

    /// Restores the previous state snapshot, pushing current state onto redo stack.
    pub fn undo(&mut self, current_state: &mut S) -> bool {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(current_state.clone());
            *current_state = prev;
            true
        } else {
            false
        }
    }

    /// Re-applies the next state snapshot, pushing current state onto undo stack.
    pub fn redo(&mut self, current_state: &mut S) -> bool {
        if let Some(next) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(current_state.clone());
            *current_state = next;
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Clone, PartialEq)]
    struct AppState {
        counter: i32,
        selected_tool: String,
    }

    enum AppAction {
        Increment,
        Decrement,
        SelectTool(String),
    }

    fn app_reducer(state: &mut AppState, action: &AppAction) {
        match action {
            AppAction::Increment => state.counter += 1,
            AppAction::Decrement => state.counter -= 1,
            AppAction::SelectTool(t) => state.selected_tool = t.clone(),
        }
    }

    #[test]
    fn test_redux_store_dispatch_and_subscription() {
        let initial = AppState {
            counter: 0,
            selected_tool: "Brush".to_string(),
        };
        let mut store = Store::new(initial, app_reducer);

        let notify_count = Arc::new(AtomicUsize::new(0));
        let count_clone = notify_count.clone();
        store.subscribe(move |_state| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        store.dispatch(AppAction::Increment);
        assert_eq!(store.state().counter, 1);
        assert_eq!(notify_count.load(Ordering::SeqCst), 1);

        store.dispatch(AppAction::SelectTool("Eraser".to_string()));
        assert_eq!(store.state().selected_tool, "Eraser");
        assert_eq!(notify_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_undo_manager_lifecycle() {
        let mut undo_mgr = UndoManager::<AppState>::new(5);
        let mut current = AppState {
            counter: 0,
            selected_tool: "Select".to_string(),
        };

        // Snapshot 1
        undo_mgr.record_snapshot(current.clone());
        current.counter = 10;

        // Snapshot 2
        undo_mgr.record_snapshot(current.clone());
        current.counter = 20;

        assert_eq!(current.counter, 20);
        assert!(undo_mgr.can_undo());
        assert!(!undo_mgr.can_redo());

        // Undo to 10
        assert!(undo_mgr.undo(&mut current));
        assert_eq!(current.counter, 10);
        assert!(undo_mgr.can_redo());

        // Undo to 0
        assert!(undo_mgr.undo(&mut current));
        assert_eq!(current.counter, 0);

        // Redo to 10
        assert!(undo_mgr.redo(&mut current));
        assert_eq!(current.counter, 10);

        // Redo to 20
        assert!(undo_mgr.redo(&mut current));
        assert_eq!(current.counter, 20);
        assert!(!undo_mgr.can_redo());
    }

    #[test]
    fn test_undo_manager_capacity_limit() {
        let mut undo_mgr = UndoManager::<i32>::new(3);
        let mut state = 0;

        for i in 1..=5 {
            undo_mgr.record_snapshot(state);
            state = i;
        }

        assert_eq!(undo_mgr.undo_count(), 3);
    }
}
