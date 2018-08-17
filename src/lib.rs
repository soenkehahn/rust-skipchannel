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
//! let (writer, mut reader) = skipchannel(0);
//! let thread = std::thread::spawn(move || {
//!   std::thread::sleep(std::time::Duration::new(0, 100_000_000));
//!   reader.last()
//! });
//! writer.write(1);
//! assert_eq!(thread.join().unwrap(), 1);
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
    last: T,
}

impl<T: Clone> Reader<T> {
    pub fn last(&mut self) -> T {
        for t in self.receiver.try_iter() {
            self.last = t.clone();
        }
        self.last.clone()
    }
}

/// Creates a [Writer](struct.Writer.html) and [Reader](struct.Reader.html)
/// for your skipchannel.
pub fn skipchannel<T>(initial: T) -> (Writer<T>, Reader<T>) {
    let (sender, receiver) = channel();
    (
        Writer { sender },
        Reader {
            receiver,
            last: initial,
        },
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
        fn allows_to_send_values_between_threads() {
            let (writer, mut reader) = skipchannel("");
            writer.write("foo");
            let read_result = parallel(move || reader.last());
            assert_eq!(read_result, "foo");
        }

        #[test]
        fn yields_initial_value_when_nothing_is_sent() {
            let (_writer, mut reader): (Writer<i32>, Reader<i32>) = skipchannel(0);
            assert_eq!(reader.last(), 0);
        }

        #[test]
        fn skips_all_values_but_the_last() {
            let (writer, mut reader) = skipchannel("");
            writer.write("foo");
            writer.write("bar");
            let read_result = parallel(move || reader.last());
            assert_eq!(read_result, "bar");
        }
    }
}
