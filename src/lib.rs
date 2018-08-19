//! This crate allows you to create a skipchannel and use it to send
//! values between threads. When you read from a skipchannel you'll only
//! ever get the last sent value, i.e. the channel skips all intermediate
//! values.
//!
//! Here's an example:
//!
//! ```
//! extern crate skipchannel;
//!
//! use skipchannel::skipchannel;
//!
//! let (sender, mut receiver) = skipchannel();
//! let thread = std::thread::spawn(move || {
//!   std::thread::sleep(std::time::Duration::new(0, 100_000_000));
//!   receiver.recv()
//! });
//! sender.send(1);
//! assert_eq!(thread.join().unwrap(), Some(1));
//! ```
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

#[derive(Debug)]
pub struct Sender<T> {
    sender: mpsc::Sender<T>,
    ptr: Arc<AtomicPtr<Option<T>>>,
}

impl<T> Sender<T> {
    pub fn send(&self, t: T) {
        self.ptr.swap(&mut Some(t), Ordering::Relaxed);
    }
}

pub struct Receiver<T> {
    ptr: Arc<AtomicPtr<Option<T>>>,
}

impl<T: Clone> Receiver<T> {
    /// Returns the last sent value. Returns `None` if
    /// no value was sent since the last call to `recv`.
    pub fn recv(&self) -> Option<T> {
        let a: &AtomicPtr<Option<T>> = &*self.ptr;
        let r = a.swap(&mut None, Ordering::Relaxed);
        let s: &mut Option<T> = unsafe { &mut *r };
        s.clone()
    }
}

/// Creates a [Sender](struct.Sender.html) and [Receiver](struct.Receiver.html)
/// for your skipchannel.
pub fn skipchannel<T>() -> (Sender<T>, Receiver<T>) {
    let (sender, receiver) = mpsc::channel();
    let ptr = Arc::new(AtomicPtr::new(&mut None));
    (
        Sender {
            sender,
            ptr: ptr.clone(),
        },
        Receiver { ptr },
    )
}

#[cfg(test)]
mod tests {
    use *;

    mod skip_channel {
        use super::*;

        fn parallel<F, T>(f: F) -> T
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static,
        {
            let f_handle = std::thread::spawn(f);
            match f_handle.join() {
                Ok(t) => t,
                Err(err) => panic!(err),
            }
        }

        #[test]
        fn allows_to_send_one_value() {
            let (sender, receiver) = skipchannel();
            sender.send("foo");
            assert_eq!(receiver.recv(), Some("foo"));
        }

        #[test]
        fn allows_to_send_values_between_threads() {
            let (sender, receiver) = skipchannel();
            sender.send("foo");
            let read_result = parallel(move || receiver.recv());
            assert_eq!(read_result, Some("foo"));
        }

        #[test]
        fn yields_none_when_nothing_is_sent() {
            let (_sender, receiver): (Sender<i32>, Receiver<i32>) = skipchannel();
            assert_eq!(receiver.recv(), None);
        }

        #[test]
        fn skips_all_values_but_the_last() {
            let (sender, receiver) = skipchannel();
            sender.send("foo");
            sender.send("bar");
            let read_result = parallel(move || receiver.recv());
            assert_eq!(read_result, Some("bar"));
        }

        #[test]
        fn returns_none_when_a_sent_value_is_already_consumed() {
            let (sender, receiver) = skipchannel();
            sender.send("foo");
            receiver.recv();
            assert_eq!(receiver.recv(), None);
        }
    }
}
