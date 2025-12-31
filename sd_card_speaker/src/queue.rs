use core::{
    array,
    cell::UnsafeCell,
    slice,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_println::println;

pub struct Queue<const N: usize> {
    buffer: [UnsafeCell<u8>; N],
    write_position: AtomicUsize,
    write_cycle: AtomicBool,
    write_signal: Signal<CriticalSectionRawMutex, ()>,
    read_position: AtomicUsize,
    read_cycle: AtomicBool,
    read_signal: Signal<CriticalSectionRawMutex, ()>,
}

impl<const N: usize> Queue<N> {
    pub fn new() -> Self {
        Self {
            buffer: array::from_fn(|_| Default::default()),
            write_position: AtomicUsize::new(0),
            write_cycle: AtomicBool::new(false),
            write_signal: Default::default(),
            read_position: AtomicUsize::new(0),
            read_cycle: AtomicBool::new(false),
            read_signal: Default::default(),
        }
    }

    pub fn split(&mut self) -> (Writer<'_, N>, Reader<'_, N>) {
        (Writer { buffer: self }, Reader { buffer: self })
    }
}

pub struct Writer<'a, const N: usize> {
    buffer: &'a Queue<N>,
}

impl<const N: usize> Writer<'_, N> {
    pub fn write_buffer(&mut self) -> (&mut [u8], &mut [u8]) {
        let write_position = self.buffer.write_position.load(Ordering::Relaxed);
        let write_cycle = self.buffer.write_cycle.load(Ordering::Relaxed);
        let read_position = self.buffer.read_position.load(Ordering::Relaxed);
        let read_cycle = self.buffer.read_cycle.load(Ordering::Relaxed);
        if write_cycle == read_cycle {
            (
                {
                    let slice = &self.buffer.buffer[write_position..];
                    let data = slice.as_ptr().cast_mut().cast();
                    let len = slice.len();
                    unsafe { slice::from_raw_parts_mut(data, len) }
                },
                {
                    let slice = &self.buffer.buffer[..read_position];
                    let data = slice.as_ptr().cast_mut().cast();
                    let len = slice.len();
                    unsafe { slice::from_raw_parts_mut(data, len) }
                },
            )
        } else {
            let slice = &self.buffer.buffer[write_position..read_position];
            let data = slice.as_ptr().cast_mut().cast();
            let len = slice.len();
            (unsafe { slice::from_raw_parts_mut(data, len) }, &mut [])
        }
    }

    pub fn mark_written(&mut self, len: usize) {
        let write_position = self.buffer.write_position.load(Ordering::Relaxed);
        let write_cycle = self.buffer.write_cycle.load(Ordering::Relaxed);
        let read_position = self.buffer.read_position.load(Ordering::Relaxed);
        let read_cycle = self.buffer.read_cycle.load(Ordering::Relaxed);
        let (new_write_position, new_write_cycle) = if write_cycle == read_cycle {
            if let Some(new_write_position) = write_position.checked_add(len)
                && new_write_position < self.buffer.buffer.len()
            {
                (new_write_position, write_cycle)
            } else {
                let new_write_position = len - (self.buffer.buffer.len() - write_position);
                // println!(
                //     "write_position: {write_position}. write_cycle: {write_cycle}. read_position: {read_position}. read_cycle: {read_cycle}. len: {len}."
                // );
                assert!(new_write_position <= read_position);
                (new_write_position, !write_cycle)
            }
        } else {
            let new_write_position = write_position + len;
            assert!(new_write_position <= read_position);
            (new_write_position, write_cycle)
        };
        self.buffer
            .write_position
            .store(new_write_position, Ordering::Relaxed);
        self.buffer
            .write_cycle
            .store(new_write_cycle, Ordering::Release);
        if
        /*(write_position == read_position && write_cycle == read_cycle) &&*/
        len > 0 {
            // println!("signaling write signal");
            self.buffer.write_signal.signal(());
        };
    }

    /// Get the number of bytes available to write
    pub fn bytes_available(&self) -> usize {
        let write_position = self.buffer.write_position.load(Ordering::Relaxed);
        let write_cycle = self.buffer.write_cycle.load(Ordering::Relaxed);
        let read_position = self.buffer.read_position.load(Ordering::Relaxed);
        let read_cycle = self.buffer.read_cycle.load(Ordering::Relaxed);
        if write_cycle == read_cycle {
            N - write_position + read_position
        } else {
            read_position - write_position
        }
    }

    /// Wait until there are at least `len` bytes of space to write to the queue
    pub async fn wait_until_available(&mut self, len: usize) {
        while self.bytes_available() < len {
            self.buffer.read_signal.wait().await;
        }
    }
}

pub struct Reader<'a, const N: usize> {
    buffer: &'a Queue<N>,
}

impl<const N: usize> Reader<'_, N> {
    pub fn read_buffer(&self) -> (&[u8], &[u8]) {
        let write_position = self.buffer.write_position.load(Ordering::Relaxed);
        let write_cycle = self.buffer.write_cycle.load(Ordering::Acquire);
        let read_position = self.buffer.read_position.load(Ordering::Relaxed);
        let read_cycle = self.buffer.read_cycle.load(Ordering::Relaxed);
        if write_cycle == read_cycle {
            let slice = &self.buffer.buffer[read_position..write_position];
            let data = slice.as_ptr().cast();
            let len = slice.len();
            (unsafe { slice::from_raw_parts(data, len) }, &[])
        } else {
            (
                {
                    let slice = &self.buffer.buffer[read_position..];
                    let data = slice.as_ptr().cast();
                    let len = slice.len();
                    unsafe { slice::from_raw_parts(data, len) }
                },
                {
                    let slice = &self.buffer.buffer[..write_position];
                    let data = slice.as_ptr().cast();
                    let len = slice.len();
                    unsafe { slice::from_raw_parts(data, len) }
                },
            )
        }
    }

    pub fn mark_read(&mut self, len: usize) {
        let write_position = self.buffer.write_position.load(Ordering::Relaxed);
        let write_cycle = self.buffer.write_cycle.load(Ordering::Relaxed);
        let read_position = self.buffer.read_position.load(Ordering::Relaxed);
        let read_cycle = self.buffer.read_cycle.load(Ordering::Relaxed);
        let (new_read_position, new_read_cycle) = if write_cycle == read_cycle {
            let new_read_position = read_position + len;
            assert!(new_read_position <= write_position);
            (new_read_position, read_cycle)
        } else {
            if let Some(new_read_position) = read_position.checked_add(len)
                && new_read_position < self.buffer.buffer.len()
            {
                (new_read_position, read_cycle)
            } else {
                let new_read_position = len - (self.buffer.buffer.len() - read_position);
                assert!(new_read_position <= write_position);
                (new_read_position, !read_cycle)
            }
        };
        self.buffer
            .read_position
            .store(new_read_position, Ordering::Relaxed);
        self.buffer
            .read_cycle
            .store(new_read_cycle, Ordering::Relaxed);
        if (read_position == write_position && read_cycle != write_cycle) && len > 0 {
            // println!("signaling read signal");
            self.buffer.read_signal.signal(());
        }
    }

    pub fn bytes_available(&self) -> usize {
        let write_position = self.buffer.write_position.load(Ordering::Relaxed);
        let write_cycle = self.buffer.write_cycle.load(Ordering::Relaxed);
        let read_position = self.buffer.read_position.load(Ordering::Relaxed);
        let read_cycle = self.buffer.read_cycle.load(Ordering::Relaxed);
        if read_cycle == write_cycle {
            write_position - read_position
        } else {
            N - read_position + write_position
        }
    }

    /// Wait until there are at least `len` bytes of read in the queue
    pub async fn wait_until_available(&mut self, len: usize) {
        while self.bytes_available() < len {
            // println!("waiting for write signal");
            self.buffer.write_signal.wait().await;
            // println!("done waiting for write signal");
        }
    }
}
