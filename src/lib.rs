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
//! let (sender, receiver) = skipchannel();
//! let thread = std::thread::spawn(move || {
//!   std::thread::sleep(std::time::Duration::new(0, 100_000_000));
//!   receiver.recv()
//! });
//! sender.send(1);
//! assert_eq!(thread.join().unwrap(), Some(1));
//! ```
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;

#[derive(Debug)]
struct AtomicPtrWithDrop<T> {
    inner: AtomicPtr<T>,
}

impl<T> AtomicPtrWithDrop<T> {
    fn new(t: *mut T) -> AtomicPtrWithDrop<T> {
        AtomicPtrWithDrop {
            inner: AtomicPtr::new(t),
        }
    }
}

impl<T> Drop for AtomicPtrWithDrop<T> {
    fn drop(&mut self) {
        let raw_ptr = self.inner.load(Ordering::SeqCst);
        unsafe {
            Box::from_raw(raw_ptr);
        }
    }
}

#[derive(Debug)]
pub struct Sender<T> {
    ptr: Arc<AtomicPtrWithDrop<Option<T>>>,
}

impl<T> Sender<T> {
    pub fn send(&self, t: T) {
        let old = self
            .ptr
            .inner
            .swap(Box::into_raw(Box::new(Some(t))), Ordering::SeqCst);
        unsafe {
            Box::from_raw(old);
        }
    }
}

pub struct Receiver<T> {
    ptr: Arc<AtomicPtrWithDrop<Option<T>>>,
}

impl<T> Receiver<T> {
    /// Returns the last sent value. Returns `None` if
    /// no value was sent since the last call to `recv`.
    pub fn recv(&self) -> Option<T> {
        let raw_ptr = self
            .ptr
            .inner
            .swap(Box::into_raw(Box::new(None)), Ordering::SeqCst);
        *(unsafe { Box::from_raw(raw_ptr) })
    }
}

/// Creates a [Sender](struct.Sender.html) and [Receiver](struct.Receiver.html)
/// for your skipchannel.
pub fn skipchannel<T>() -> (Sender<T>, Receiver<T>) {
    let ptr = Arc::new(AtomicPtrWithDrop::new(Box::into_raw(Box::new(None))));
    (Sender { ptr: ptr.clone() }, Receiver { ptr })
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

        mod drop {
            use super::*;
            use std::sync::atomic::AtomicBool;

            struct DropMock {
                was_dropped: Arc<AtomicBool>,
            }

            impl DropMock {
                fn new() -> (DropMock, Arc<AtomicBool>) {
                    let arc = Arc::new(AtomicBool::new(false));
                    (
                        DropMock {
                            was_dropped: arc.clone(),
                        },
                        arc,
                    )
                }
            }

            impl<'a> Drop for DropMock {
                fn drop(&mut self) {
                    self.was_dropped.store(true, Ordering::SeqCst);
                }
            }

            #[test]
            fn drops_unconsumed_values_on_subsequent_sends() {
                let (first, first_was_dropped) = DropMock::new();
                let (sender, _receiver) = skipchannel();
                sender.send(Some(first));
                sender.send(None);
                assert_eq!(first_was_dropped.load(Ordering::SeqCst), true);
            }

            #[test]
            fn value_is_dropped_when_sender_and_receiver_are_dropped() {
                let (value, value_was_dropped) = DropMock::new();
                {
                    let (sender, _receiver) = skipchannel();
                    sender.send(value);
                }
                assert_eq!(value_was_dropped.load(Ordering::SeqCst), true);
            }

            #[test]
            fn does_not_drop_values_when_just_the_receiver_is_dropped() {
                let (value, value_was_dropped) = DropMock::new();
                let (sender, receiver) = skipchannel();
                sender.send(value);
                std::mem::drop(receiver);
                assert_eq!(
                    value_was_dropped.load(Ordering::SeqCst),
                    false,
                    "value dropped although sender is still alive"
                );
                std::mem::drop(sender);
                assert_eq!(
                    value_was_dropped.load(Ordering::SeqCst),
                    true,
                    "value not dropped"
                );
            }

            #[test]
            fn does_not_drop_values_when_just_the_sender_is_dropped() {
                let (value, value_was_dropped) = DropMock::new();
                let (sender, receiver) = skipchannel();
                sender.send(value);
                std::mem::drop(sender);
                assert_eq!(
                    value_was_dropped.load(Ordering::SeqCst),
                    false,
                    "value dropped although receiver is still alive"
                );
                std::mem::drop(receiver);
                assert_eq!(
                    value_was_dropped.load(Ordering::SeqCst),
                    true,
                    "value not dropped"
                );
            }
        }

        #[test]
        fn test_doctest() {
            let (sender, receiver) = skipchannel();
            let thread = std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::new(0, 100_000_000));
                receiver.recv()
            });
            sender.send(1);
            assert_eq!(thread.join().unwrap(), Some(1));
        }
    }
}
