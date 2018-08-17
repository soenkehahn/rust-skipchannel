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
