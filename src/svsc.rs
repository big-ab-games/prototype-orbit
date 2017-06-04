use std::sync::{Arc, Mutex, Weak};
use std::result::Result;

pub struct Getter<T> {
    latest: T,
    latest_set: Arc<Mutex<Option<T>>>
}

impl<T> Getter<T> {
    fn update_latest(&mut self) {
        if let Ok(mut latest_set) = self.latest_set.lock() {
            if let Some(value) = latest_set.take() {
                self.latest = value;
            }
        }
    }

    pub fn latest(&mut self) -> &T {
        self.update_latest();
        &self.latest
    }

    // pub fn latest_mut(&mut self) -> &mut T {
    //     self.update_latest();
    //     &mut self.latest
    // }
}

#[derive(Clone)]
pub struct Updater<T> {
    latest: Weak<Mutex<Option<T>>>
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct DeadGetterError<T>(pub T);

impl<T> Updater<T> {
    pub fn update(&self, value: T) -> Result<(), DeadGetterError<T>> {
        match self.latest.upgrade() {
            Some(mutex) => {
                *mutex.lock().unwrap() = Some(value);
                Ok(())
            }
            None => Err(DeadGetterError(value))
        }
    }

    pub fn getter_is_dead(&self) -> bool {
        self.latest.upgrade().is_none()
    }
}

pub fn channel<T: Send>(initial: T) -> (Getter<T>, Updater<T>) {
    let getter = Getter { latest: initial, latest_set: Arc::new(Mutex::new(None)) };
    let updater = Updater { latest: Arc::downgrade(&getter.latest_set) };
    (getter, updater)
}

#[cfg(test)]
mod svsc_tests {
    use super::*;
    use std::thread;
    use std::sync::Barrier;
    use std::mem;

    #[test]
    fn send_recv_value() {
        let (mut recv, send) = channel(12);
        assert_eq!(recv.latest(), &12);
        send.update(123).unwrap();
        assert_eq!(recv.latest(), &123);
    }

    #[test]
    fn send_recv_option() {
        // ensure option value works nicely
        let (mut recv, send) = channel(None);
        assert_eq!(*recv.latest(), None);
        send.update(Some(234)).unwrap();
        assert_eq!(*recv.latest(), Some(234));
    }

    fn barrier_pair() -> (Arc<Barrier>, Arc<Barrier>) {
        let barrier = Arc::new(Barrier::new(2));
        (barrier.clone(), barrier)
    }

    #[test]
    fn concurrent_send_recv() {
        let (mut recv, send) = channel(0);
        let (barrier, barrier2) = barrier_pair();

        thread::spawn(move|| {
            barrier2.wait(); // <- read initial
            for num in 1..1000 {
                send.update(num).unwrap();
            }
            send.update(1000).unwrap();

            barrier2.wait(); // <- sent 1000
            for num in 1001..2001 {
                send.update(num).unwrap();
            }
            barrier2.wait(); // <- sent 2000
        });

        let mut distinct_recvs = 1;
        let mut last_result = *recv.latest();
        barrier.wait(); // <- read initial
        while last_result < 1000 {
            let next = *recv.latest();
            if next != last_result {
                distinct_recvs += 1;
            }
            last_result = next;
        }
        assert!(distinct_recvs > 1);
        println!("received: {}", distinct_recvs);

        assert_eq!(*recv.latest(), 1000);
        barrier.wait(); // <- sent 1000
        barrier.wait(); // <- sent 2000
        assert_eq!(*recv.latest(), 2000);
    }

    #[test]
    fn non_blocking_write_during_read() {
        let (mut name_get, name) = channel("Nothing".to_owned());
        let (barrier, barrier2) = barrier_pair();
        thread::spawn(move|| {
            barrier2.wait(); // <- has read lock
            name.update("Something".to_owned()).unwrap();
            barrier2.wait(); // <- value updated
        });

        {
            let got = name_get.latest();
            assert_eq!(*got, "Nothing".to_owned());
            barrier.wait(); // <- has read lock
            barrier.wait(); // <- value updated
        }

        let got2 = name_get.latest();
        assert_eq!(*got2, "Something".to_owned());
    }

    #[test]
    fn error_writing_to_dead_reader() {
        let (val_get, val) = channel(0);
        mem::drop(val_get);
        assert_eq!(val.update(123), Err(DeadGetterError(123)));
    }

    #[test]
    fn is_alive() {
        let (val_get, val) = channel(0);
        assert!(!val.getter_is_dead());
        mem::drop(val_get);
        assert!(val.getter_is_dead());
    }
}
