use core::sync::atomic::{Atomic, AtomicUsize, Ordering};

use crate::os::beetos::ffi::{blocking_scalar, scalar};
use crate::os::beetos::services::{TicktimerScalar, ticktimer_server};
use crate::sys::sync::Mutex;
use crate::time::Duration;

const NOTIFY_TRIES: usize = 3;

pub struct Condvar {
    counter: Atomic<usize>,
    timed_out: Atomic<usize>,
}

unsafe impl Send for Condvar {}
unsafe impl Sync for Condvar {}

impl Condvar {
    #[inline]
    pub const fn new() -> Condvar {
        Condvar { counter: AtomicUsize::new(0), timed_out: AtomicUsize::new(0) }
    }

    fn notify_some(&self, to_notify: usize) {
        assert!(self.timed_out.load(Ordering::Relaxed) <= self.counter.load(Ordering::Relaxed));
        self.counter.fetch_sub(self.timed_out.swap(0, Ordering::Relaxed), Ordering::Relaxed);

        let Ok(waiter_count) =
            self.counter.try_update(Ordering::Relaxed, Ordering::Relaxed, |counter| {
                if counter == 0 {
                    return None;
                } else {
                    Some(counter - counter.min(to_notify))
                }
            })
        else {
            return;
        };

        let mut remaining_to_wake = waiter_count.min(to_notify);
        if remaining_to_wake == 0 {
            return;
        }
        for _wake_tries in 0..NOTIFY_TRIES {
            let result = blocking_scalar(
                ticktimer_server(),
                TicktimerScalar::NotifyCondition(self.index(), remaining_to_wake).into(),
            )
            .expect("failure to send NotifyCondition command");

            remaining_to_wake -= result[0];
            remaining_to_wake =
                remaining_to_wake.saturating_sub(self.timed_out.swap(0, Ordering::Relaxed));
            if remaining_to_wake == 0 {
                return;
            }
            crate::thread::yield_now();
        }
    }

    pub fn notify_one(&self) {
        self.notify_some(1)
    }

    pub fn notify_all(&self) {
        self.notify_some(self.counter.load(Ordering::Relaxed))
    }

    fn index(&self) -> usize {
        core::ptr::from_ref(self).addr()
    }

    fn wait_ms(&self, mutex: &Mutex, ms: usize) -> bool {
        self.counter.fetch_add(1, Ordering::Relaxed);
        unsafe { mutex.unlock() };

        let result = blocking_scalar(
            ticktimer_server(),
            TicktimerScalar::WaitForCondition(self.index(), ms).into(),
        );
        let awoken = result.expect("Ticktimer: failure to send WaitForCondition command")[0] == 0;

        if !awoken {
            self.timed_out.fetch_add(1, Ordering::Relaxed);
        }

        unsafe { mutex.lock() };
        awoken
    }

    pub unsafe fn wait(&self, mutex: &Mutex) {
        self.wait_ms(mutex, 0);
    }

    pub unsafe fn wait_timeout(&self, mutex: &Mutex, dur: Duration) -> bool {
        let mut millis = dur.as_millis() as usize;
        if millis == 0 {
            millis = 1;
        }
        self.wait_ms(mutex, millis)
    }
}

impl Drop for Condvar {
    fn drop(&mut self) {
        let remaining_count = self.counter.load(Ordering::Relaxed);
        let timed_out = self.timed_out.load(Ordering::Relaxed);
        assert!(
            remaining_count - timed_out == 0,
            "counter was {} and timed_out was {} not 0",
            remaining_count,
            timed_out
        );
        scalar(ticktimer_server(), TicktimerScalar::FreeCondition(self.index()).into()).ok();
    }
}
