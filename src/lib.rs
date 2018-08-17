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
//! let (writer, mut reader) = skipchannel();
//! let thread = std::thread::spawn(move || {
//!   std::thread::sleep(std::time::Duration::new(0, 100_000_000));
//!   reader.recv()
//! });
//! writer.write(1);
//! assert_eq!(thread.join().unwrap(), Some(1));
//! ```
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug)]
pub struct Writer<T> {
    sender: Sender<T>,
}

impl<T> Writer<T> {
    pub fn write(&self, t: T) {
        let _ = self.sender.send(t);
    }
}

pub struct Reader<T> {
    receiver: Receiver<T>,
}

impl<T> Reader<T> {
    /// Returns the last sent value. Returns `None` if
    /// no value was sent since the last call to `recv`.
    pub fn recv(&mut self) -> Option<T> {
        let mut result = None;
        for t in self.receiver.try_iter() {
            result = Some(t);
        }
        result
    }
}

/// Creates a [Writer](struct.Writer.html) and [Reader](struct.Reader.html)
/// for your skipchannel.
pub fn skipchannel<T>() -> (Writer<T>, Reader<T>) {
    let (sender, receiver) = channel();
    (Writer { sender }, Reader { receiver })
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
            let (writer, mut reader) = skipchannel();
            writer.write("foo");
            assert_eq!(reader.recv(), Some("foo"));
        }

        #[test]
        fn allows_to_send_values_between_threads() {
            let (writer, mut reader) = skipchannel();
            writer.write("foo");
            let read_result = parallel(move || reader.recv());
            assert_eq!(read_result, Some("foo"));
        }

        #[test]
        fn yields_none_when_nothing_is_sent() {
            let (_writer, mut reader): (Writer<i32>, Reader<i32>) = skipchannel();
            assert_eq!(reader.recv(), None);
        }

        #[test]
        fn skips_all_values_but_the_last() {
            let (writer, mut reader) = skipchannel();
            writer.write("foo");
            writer.write("bar");
            let read_result = parallel(move || reader.recv());
            assert_eq!(read_result, Some("bar"));
        }

        #[test]
        fn returns_none_when_a_sent_value_is_already_consumed() {
            let (writer, mut reader) = skipchannel();
            writer.write("foo");
            reader.recv();
            assert_eq!(reader.recv(), None);
        }
    }
}
