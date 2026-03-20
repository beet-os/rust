use crate::os::beetos::ffi::{blocking_scalar, scalar};
use crate::os::beetos::services::{TicktimerScalar, ticktimer_server};
use crate::pin::Pin;
use crate::ptr;
use crate::sync::atomic::Ordering::{Acquire, Release};
use crate::sync::atomic::{Atomic, AtomicI8};
use crate::time::Duration;

const NOTIFIED: i8 = 1;
const EMPTY: i8 = 0;
const PARKED: i8 = -1;

pub struct Parker {
    state: Atomic<i8>,
}

impl Parker {
    pub unsafe fn new_in_place(parker: *mut Parker) {
        unsafe { parker.write(Parker { state: AtomicI8::new(EMPTY) }) }
    }

    fn index(&self) -> usize {
        ptr::from_ref(self).addr()
    }

    pub unsafe fn park(self: Pin<&Self>) {
        let state = self.state.fetch_sub(1, Acquire);
        if state == NOTIFIED {
            return;
        }
        assert!(state == EMPTY);

        blocking_scalar(
            ticktimer_server(),
            TicktimerScalar::WaitForCondition(self.index(), 0).into(),
        )
        .expect("failed to send WaitForCondition command");

        let state = self.state.swap(EMPTY, Acquire);
        assert!(state == NOTIFIED || state == PARKED);
    }

    pub unsafe fn park_timeout(self: Pin<&Self>, timeout: Duration) {
        let state = self.state.fetch_sub(1, Acquire);
        if state == NOTIFIED {
            return;
        }
        assert!(state == EMPTY);

        let millis = usize::max(timeout.as_millis().try_into().unwrap_or(usize::MAX), 1);

        let _was_timeout = blocking_scalar(
            ticktimer_server(),
            TicktimerScalar::WaitForCondition(self.index(), millis).into(),
        )
        .expect("failed to send WaitForCondition command")[0]
            != 0;

        let state = self.state.swap(EMPTY, Acquire);
        assert!(state == PARKED || state == NOTIFIED);
    }

    pub fn unpark(self: Pin<&Self>) {
        if self.state.swap(NOTIFIED, Release) != PARKED {
            return;
        }

        while blocking_scalar(
            ticktimer_server(),
            TicktimerScalar::NotifyCondition(self.index(), 1).into(),
        )
        .expect("failed to send NotifyCondition command")[0]
            == 0
            && self.state.load(Acquire) != EMPTY
        {
            crate::thread::yield_now();
        }
    }
}

impl Drop for Parker {
    fn drop(&mut self) {
        scalar(ticktimer_server(), TicktimerScalar::FreeCondition(self.index()).into()).ok();
    }
}
