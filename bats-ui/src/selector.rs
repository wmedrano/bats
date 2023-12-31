use std::marker::PhantomData;

/// Helps manage selection from a list.
pub struct Selector<T, A: AsRef<[T]>> {
    items: A,
    selected: usize,
    _data_type: PhantomData<T>,
}

impl<T, A: AsRef<[T]>> Selector<T, A> {
    /// Create a new selector that points to the first item of `items`. `items` must not be empty.
    pub fn new(items: A) -> Self {
        assert!(!items.as_ref().is_empty());
        Selector {
            items,
            selected: 0,
            _data_type: PhantomData,
        }
    }

    /// Iterate over all items. The iterator contains `(true_if_selected, &item)`.
    pub fn iter(&self) -> impl Iterator<Item = (bool, &T)> {
        self.items
            .as_ref()
            .iter()
            .enumerate()
            .map(|(idx, item)| (idx == self.selected, item))
    }

    /// Return a reference to the currently selected item.
    pub fn selected(&self) -> &T {
        &self.items.as_ref()[self.selected]
    }

    /// Advance the selection by `pos`. If `pos` is negative, then the selection moves backwards.
    ///
    /// Note: Selection wraps around.
    pub fn select_by(&mut self, pos: isize) {
        if pos < 0 {
            self.select_by(pos + self.items.as_ref().len() as isize);
        } else {
            self.selected = (self.selected + pos as usize).rem_euclid(self.items.as_ref().len());
        }
    }
}
