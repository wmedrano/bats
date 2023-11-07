/// Helps manage selection from a list.
pub struct Selector<T> {
    items: Vec<T>,
    selected: usize,
}

impl<'a, T> Selector<T> {
    /// Create a new selector that points to the first item of `items`. `items` must not be empty.
    pub fn new(items: Vec<T>) -> Self {
        assert!(!items.is_empty());
        Selector { items, selected: 0 }
    }

    /// Iterate over all items. The iterator contains `(true_if_selected, &item)`.
    pub fn iter(&'a self) -> impl Iterator<Item = (bool, &'a T)> {
        self.items
            .iter()
            .enumerate()
            .map(|(idx, item)| (idx == self.selected, item))
    }

    /// Return a reference to the currently selected item.
    pub fn selected(&self) -> &T {
        &self.items[self.selected]
    }

    /// Advance the selection by `pos`. If `pos` is negative, then the selection moves backwards.
    ///
    /// Note: Selection wraps around.
    pub fn select_by(&mut self, pos: isize) {
        if pos < 0 {
            self.select_by(pos + self.items.len() as isize);
        } else {
            self.selected = (self.selected + pos as usize).rem_euclid(self.items.len());
        }
    }
}
