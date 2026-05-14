use split_slice::SplitSlice;

pub struct Buffer<T, const N: usize> {
    buffer: [T; N],
    saved_start: usize,
    saved_len: usize,
    start: usize,
    len: usize,
}

impl<T: Default + Copy, const N: usize> Buffer<T, N> {
    pub fn new() -> Self {
        Self {
            buffer: [Default::default(); N],
            saved_start: 0,
            saved_len: 0,
            start: 0,
            len: 0,
        }
    }

    pub fn free_slots_mut(&mut self) -> &mut [T] {
        &mut self.buffer[self.start + self.len..]
    }

    pub fn extend_len(&mut self, count: usize) {
        self.len += count;
    }

    pub fn slices(&self) -> SplitSlice<'_, T> {
        if self.start + self.len > self.buffer.len() {
            SplitSlice(
                &self.buffer[self.start..],
                &self.buffer[..self.start + self.len - self.buffer.len()],
            )
        } else {
            SplitSlice(&self.buffer[self.start..self.start + self.len], &[])
        }
    }

    pub fn shift(&mut self, count: usize, save: bool) {
        self.start = (self.start + count) % self.buffer.len();
        if save {
            self.saved_len += count;
        }
        self.len -= count
    }

    pub fn clear(&mut self) {
        self.start = self.saved_start + self.saved_len;
        self.len = 0;
    }

    pub fn saved(&self) -> SplitSlice<'_, T> {
        if self.saved_start + self.saved_len > self.buffer.len() {
            SplitSlice(
                &self.buffer[self.saved_start..],
                &self.buffer[..self.saved_start + self.len - self.buffer.len()],
            )
        } else {
            SplitSlice(
                &self.buffer[self.saved_start..self.saved_start + self.len],
                &[],
            )
        }
    }

    pub fn clear_saved(&mut self) {
        self.saved_len = 0;
    }
}
