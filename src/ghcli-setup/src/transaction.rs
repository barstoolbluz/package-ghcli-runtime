/// Drop-based transaction guard for rollback on error.
///
/// Push rollback closures before each mutation. On success, call `commit()`.
/// If the guard is dropped without committing, all rollback actions fire in
/// reverse order. This replaces the fragile backup/restore globals from the
/// bash implementation.
pub struct TransactionGuard {
    rollback_actions: Vec<Box<dyn FnOnce()>>,
    committed: bool,
}

impl TransactionGuard {
    pub fn new() -> Self {
        Self {
            rollback_actions: Vec::new(),
            committed: false,
        }
    }

    /// Register a rollback action to be executed if the transaction is not committed.
    pub fn push_rollback<F: FnOnce() + 'static>(&mut self, action: F) {
        self.rollback_actions.push(Box::new(action));
    }

    /// Mark the transaction as committed. Rollback actions will not fire.
    pub fn commit(mut self) {
        self.committed = true;
        self.rollback_actions.clear();
    }
}

impl Drop for TransactionGuard {
    fn drop(&mut self) {
        if !self.committed {
            // Execute rollback actions in reverse order
            for action in self.rollback_actions.drain(..).rev() {
                action();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_committed_no_rollback() {
        let ran = Rc::new(RefCell::new(false));
        let ran_clone = ran.clone();
        {
            let mut guard = TransactionGuard::new();
            guard.push_rollback(move || {
                *ran_clone.borrow_mut() = true;
            });
            guard.commit();
        }
        assert!(!*ran.borrow());
    }

    #[test]
    fn test_dropped_runs_rollback() {
        let ran = Rc::new(RefCell::new(false));
        let ran_clone = ran.clone();
        {
            let mut guard = TransactionGuard::new();
            guard.push_rollback(move || {
                *ran_clone.borrow_mut() = true;
            });
            // Drop without committing
        }
        assert!(*ran.borrow());
    }

    #[test]
    fn test_reverse_order() {
        let order = Rc::new(RefCell::new(Vec::new()));
        let o1 = order.clone();
        let o2 = order.clone();
        let o3 = order.clone();
        {
            let mut guard = TransactionGuard::new();
            guard.push_rollback(move || o1.borrow_mut().push(1));
            guard.push_rollback(move || o2.borrow_mut().push(2));
            guard.push_rollback(move || o3.borrow_mut().push(3));
        }
        assert_eq!(*order.borrow(), vec![3, 2, 1]);
    }
}
