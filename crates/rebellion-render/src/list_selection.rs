//! The multiple selection of the original's list control, `CoolDragList`
//! (`FUN_006083c0`), shared by the windows whose lists use it.
//!
//! A left or right press (`0x201`/`0x204`) on an item:
//!
//! - already selected: nothing changes yet, and the press is pending
//!   (`+0xf4` bit `0x10`); a release with no drag applies the rule below
//!   then (case `0x202`), so dragging a selected item carries the whole
//!   selection;
//! - with Ctrl (`MK_CONTROL`, 8): the item's selected bit (`+0x3c` bit 0)
//!   toggles;
//! - with Shift (`MK_SHIFT`, 4): every item whose cell meets the rectangle
//!   between the anchor and the press is added (`FUN_00609590`);
//! - otherwise: the selection is cleared (`FUN_006094b0`) and the item is
//!   selected.
//!
//! The press point becomes the anchor (`+0xf8`, `+0xf4` bit 2) either way.
//! A press on no item clears the selection, except with Ctrl, which keeps
//! it. port: a Shift press on no item keeps the selection too; the
//! original adds the rectangle to the empty point.

/// The modifier keys `FUN_006083c0` reads from the message's `wParam`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
}

/// A list's selection, in the list's own order of pressing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListSelection<T> {
    items: Vec<T>,
    /// The cell of the last press: (column, row) in the list's grid.
    anchor: Option<(usize, usize)>,
    /// A press on a selected item, applied at its release.
    pending: Option<T>,
}

impl<T> Default for ListSelection<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            anchor: None,
            pending: None,
        }
    }
}

impl<T: Copy + PartialEq> ListSelection<T> {
    /// A selection of just `item`.
    #[must_use]
    pub fn only(item: T) -> Self {
        Self {
            items: vec![item],
            ..Self::default()
        }
    }

    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.items
    }

    #[must_use]
    pub fn contains(&self, item: T) -> bool {
        self.items.contains(&item)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.anchor = None;
        self.pending = None;
    }

    /// Drop the items `keep` rejects (a refill removed them).
    pub fn retain(&mut self, mut keep: impl FnMut(T) -> bool) {
        self.items.retain(|&item| keep(item));
        if self.pending.is_some_and(|item| !keep(item)) {
            self.pending = None;
        }
    }

    /// A press on `rows[index]` of a grid `columns` wide.
    pub fn press(&mut self, rows: &[T], columns: usize, index: usize, modifiers: Modifiers) {
        let Some(&item) = rows.get(index) else {
            return;
        };
        if self.contains(item) {
            self.pending = Some(item);
        } else {
            self.pending = None;
            self.apply(rows, columns, index, modifiers);
        }
        self.anchor = Some(cell(columns, index));
    }

    /// The release that ends a press with no drag: a pending press applies
    /// its rule now.
    pub fn release(&mut self, rows: &[T], columns: usize, index: usize, modifiers: Modifiers) {
        let Some(&item) = rows.get(index) else {
            return;
        };
        if self.pending.take() == Some(item) {
            self.apply(rows, columns, index, modifiers);
        }
    }

    /// A press on the list away from every item.
    pub fn press_empty(&mut self, modifiers: Modifiers) {
        self.pending = None;
        if !modifiers.ctrl && !modifiers.shift {
            self.items.clear();
        }
    }

    /// The items a drag pressed on `item` carries: the selection when the
    /// item is in it, else the selection the press makes.
    #[must_use]
    pub fn dragged(&self, item: T, modifiers: Modifiers) -> Vec<T> {
        if self.contains(item) {
            self.items.clone()
        } else if modifiers.ctrl || modifiers.shift {
            let mut items = self.items.clone();
            items.push(item);
            items
        } else {
            vec![item]
        }
    }

    fn apply(&mut self, rows: &[T], columns: usize, index: usize, modifiers: Modifiers) {
        let item = rows[index];
        if modifiers.ctrl {
            if let Some(at) = self.items.iter().position(|&value| value == item) {
                self.items.remove(at);
            } else {
                self.items.push(item);
            }
        } else if modifiers.shift {
            let anchor = self.anchor.unwrap_or_else(|| cell(columns, index));
            let here = cell(columns, index);
            let (left, right) = (anchor.0.min(here.0), anchor.0.max(here.0));
            let (top, bottom) = (anchor.1.min(here.1), anchor.1.max(here.1));
            for (at, &value) in rows.iter().enumerate() {
                let (column, row) = cell(columns, at);
                if (left..=right).contains(&column)
                    && (top..=bottom).contains(&row)
                    && !self.items.contains(&value)
                {
                    self.items.push(value);
                }
            }
        } else {
            self.items.clear();
            self.items.push(item);
        }
    }
}

const fn cell(columns: usize, index: usize) -> (usize, usize) {
    let columns = if columns == 0 { 1 } else { columns };
    (index % columns, index / columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROWS: [u8; 7] = [10, 11, 12, 13, 14, 15, 16];
    const PLAIN: Modifiers = Modifiers {
        ctrl: false,
        shift: false,
    };
    const CTRL: Modifiers = Modifiers {
        ctrl: true,
        shift: false,
    };
    const SHIFT: Modifiers = Modifiers {
        ctrl: false,
        shift: true,
    };

    fn click(selection: &mut ListSelection<u8>, index: usize, modifiers: Modifiers) {
        selection.press(&ROWS, 3, index, modifiers);
        selection.release(&ROWS, 3, index, modifiers);
    }

    /// `FUN_006083c0` case `0x201`: a plain press clears the others
    /// (`FUN_006094b0`) and selects the item; Ctrl toggles it.
    #[test]
    fn a_plain_press_selects_one_and_ctrl_adds_or_removes_it() {
        let mut selection = ListSelection::default();
        click(&mut selection, 0, PLAIN);
        click(&mut selection, 2, CTRL);
        click(&mut selection, 4, CTRL);
        assert_eq!(selection.items(), [10, 12, 14]);
        click(&mut selection, 2, CTRL);
        assert_eq!(selection.items(), [10, 14]);
        click(&mut selection, 5, PLAIN);
        assert_eq!(selection.items(), [15]);
    }

    /// `FUN_00609590` adds every cell inside the rectangle from the anchor
    /// to the press: in a grid three wide, cells 1 and 5 span columns 1-2
    /// and rows 0-1.
    #[test]
    fn shift_adds_the_cells_of_the_box_from_the_anchor() {
        let mut selection = ListSelection::default();
        click(&mut selection, 6, PLAIN);
        click(&mut selection, 1, PLAIN);
        click(&mut selection, 5, SHIFT);
        assert_eq!(selection.items(), [11, 12, 14, 15]);
    }

    /// A press on a selected item waits for its release (`+0xf4` bit
    /// `0x10`), so a drag from it carries the whole selection; the release
    /// without a drag then selects the item alone.
    #[test]
    fn a_press_on_the_selection_keeps_it_for_a_drag_until_the_release() {
        let mut selection = ListSelection::default();
        click(&mut selection, 0, PLAIN);
        click(&mut selection, 1, CTRL);
        selection.press(&ROWS, 3, 1, PLAIN);
        assert_eq!(selection.items(), [10, 11]);
        assert_eq!(selection.dragged(11, PLAIN), [10, 11]);
        selection.release(&ROWS, 3, 1, PLAIN);
        assert_eq!(selection.items(), [11]);
        assert_eq!(selection.dragged(13, PLAIN), [13]);
        assert_eq!(selection.dragged(13, CTRL), [11, 13]);
    }

    /// A press on no item clears the selection; with Ctrl it keeps it
    /// (`FUN_006083c0`: no item, `goto` past the change).
    #[test]
    fn a_press_on_empty_space_clears_unless_ctrl_is_held() {
        let mut selection = ListSelection::default();
        click(&mut selection, 0, PLAIN);
        click(&mut selection, 1, CTRL);
        selection.press_empty(CTRL);
        assert_eq!(selection.items(), [10, 11]);
        assert!(!selection.is_empty(), "Ctrl keeps the selection");
        selection.press_empty(PLAIN);
        assert!(selection.is_empty());
    }

    /// The rows sit in a grid, so the box spans whole rows and columns, not
    /// a run of list positions: in a grid three wide, the cells from 4 to 6
    /// cover columns 0-1 of rows 1-2 and leave row 0 out.
    #[test]
    fn the_shift_box_counts_rows_down_the_grid() {
        let mut selection = ListSelection::default();
        click(&mut selection, 4, PLAIN);
        click(&mut selection, 6, SHIFT);
        assert_eq!(selection.items(), [14, 13, 16]);
    }

    /// A refill drops the rows that left the page (`retain`), and with them a
    /// press waiting on one; a press on a row that stays keeps waiting, so
    /// its release still applies the press's rule.
    #[test]
    fn a_refill_drops_the_rows_that_left_and_any_press_waiting_on_one() {
        let mut selection = ListSelection::default();
        click(&mut selection, 0, PLAIN);
        click(&mut selection, 1, CTRL);

        selection.press(&ROWS, 3, 1, PLAIN);
        selection.retain(|item| item != 11);
        assert_eq!(selection.items(), [10], "the row that left is dropped");
        selection.release(&ROWS, 3, 1, PLAIN);
        assert_eq!(selection.items(), [10], "its waiting press went with it");

        click(&mut selection, 1, CTRL);
        selection.press(&ROWS, 3, 1, PLAIN);
        selection.retain(|_| true);
        selection.release(&ROWS, 3, 1, PLAIN);
        assert_eq!(selection.items(), [11], "a kept row's press still settles");
    }
}
