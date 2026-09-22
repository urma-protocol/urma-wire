use crate::limits::MAX_DISPLAY_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget(usize);

pub enum Room {
    Fits(Budget),
    Exhausted,
}

impl Budget {
    pub const EMPTY: Budget = Budget(0);

    pub fn spent(self) -> usize {
        self.0
    }

    pub fn of(spent: usize) -> Self {
        Budget(spent.min(MAX_DISPLAY_BYTES))
    }

    pub fn full(self) -> bool {
        self.0 >= MAX_DISPLAY_BYTES
    }

    pub fn holding(self, bytes: usize) -> Room {
        let Some(total) = self.0.checked_add(bytes) else {
            return Room::Exhausted;
        };
        match total <= MAX_DISPLAY_BYTES {
            true => Room::Fits(Budget(total)),
            false => Room::Exhausted,
        }
    }

    pub fn releasing(self, bytes: usize) -> Self {
        let Some(left) = self.0.checked_sub(bytes) else {
            return Budget::EMPTY;
        };
        Budget(left)
    }
}
