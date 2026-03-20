use core::sync::atomic::{Atomic, AtomicU32, Ordering};

use crate::os::beetos::ffi::Connection;

fn group_or_null(data: &[u8], offset: usize) -> usize {
    let start = offset * size_of::<usize>();
    let mut out_array = [0u8; size_of::<usize>()];
    if start < data.len() {
        for (dest, src) in out_array.iter_mut().zip(&data[start..]) {
            *dest = *src;
        }
    }
    usize::from_le_bytes(out_array)
}

pub(crate) enum LogScalar<'a> {
    BeginPanic,
    AppendPanicMessage(&'a [u8]),
}

impl<'a> Into<[usize; 5]> for LogScalar<'a> {
    fn into(self) -> [usize; 5] {
        match self {
            LogScalar::BeginPanic => [1000, 0, 0, 0, 0],
            LogScalar::AppendPanicMessage(c) => [
                1100 + c.len(),
                group_or_null(&c, 0),
                group_or_null(&c, 1),
                group_or_null(&c, 2),
                group_or_null(&c, 3),
            ],
        }
    }
}

pub(crate) enum LogLend {
    StandardOutput = 1,
    StandardError = 2,
}

impl Into<usize> for LogLend {
    fn into(self) -> usize {
        self as usize
    }
}

pub(crate) fn log_server() -> Connection {
    static LOG_SERVER_CONNECTION: Atomic<u32> = AtomicU32::new(0);

    let cid = LOG_SERVER_CONNECTION.load(Ordering::Relaxed);
    if cid != 0 {
        return cid.into();
    }

    let cid = crate::os::beetos::ffi::connect("xous-log-server ".try_into().unwrap()).unwrap();
    LOG_SERVER_CONNECTION.store(cid.into(), Ordering::Relaxed);
    cid
}
