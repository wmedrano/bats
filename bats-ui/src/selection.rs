/// Manage selection of menu items.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct MenuSelection {
    /// The selected index.
    pub selected_idx: usize,
}

impl MenuSelection {
    /// Get the currently selected item from `items`.
    pub fn selection<T>(&self, items: impl Iterator<Item = T>) -> Option<T> {
        let mut items = items;
        items.nth(self.selected_idx)
    }

    /// Move the selection by `n`. If `n` is negative, the selection will move backwards.
    pub fn move_selection(&mut self, n: isize, items_count: usize) {
        let items_count = items_count as isize;
        let idx = (self.selected_idx as isize + n) % items_count;
        if idx < 0 {
            self.selected_idx = (idx + items_count) as usize;
        } else {
            self.selected_idx = idx as usize;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_selects_right_item() {
        let s = MenuSelection { selected_idx: 5 };
        assert_eq!(
            s.selection(0..10).map(|i| i.to_string()),
            Some("5".to_string())
        );
        assert_eq!(s.selection(0..2).map(|i| i.to_string()), None);
    }

    #[test]
    fn move_selection_advances_selection() {
        let mut state = MenuSelection { selected_idx: 1 };
        state.move_selection(2, 100);
        assert_eq!(state.selected_idx, 3);
    }

    #[test]
    fn move_selection_wraps_around() {
        let mut state = MenuSelection { selected_idx: 1 };
        state.move_selection(3, 4);
        assert_eq!(state.selected_idx, 0);
    }

    #[test]
    fn move_selection_addvances_selection_backward() {
        let mut state = MenuSelection { selected_idx: 2 };
        state.move_selection(-1, 100);
        assert_eq!(state.selected_idx, 1);
    }

    #[test]
    fn move_selection_wraps_around_backwards() {
        let mut state = MenuSelection { selected_idx: 2 };
        state.move_selection(-3, 4);
        assert_eq!(state.selected_idx, 3);
    }
}
