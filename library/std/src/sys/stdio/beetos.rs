#[expect(dead_code)]
#[path = "unsupported.rs"]
mod unsupported_stdio;

use crate::io;
use crate::os::beetos::ffi::{Connection, lend, try_scalar};
use crate::os::beetos::services::{LogLend, LogScalar, log_server};

pub type Stdin = unsupported_stdio::Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        #[repr(C, align(16384))]
        struct LendBuffer([u8; 16384]);
        let mut lend_buffer = LendBuffer([0u8; 16384]);
        let connection = log_server();
        for chunk in buf.chunks(lend_buffer.0.len()) {
            for (dest, src) in lend_buffer.0.iter_mut().zip(chunk) {
                *dest = *src;
            }
            lend(connection, LogLend::StandardOutput.into(), &lend_buffer.0, 0, chunk.len())
                .unwrap();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Stderr {
    pub const fn new() -> Stderr {
        Stderr
    }
}

impl io::Write for Stderr {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        #[repr(C, align(16384))]
        struct LendBuffer([u8; 16384]);
        let mut lend_buffer = LendBuffer([0u8; 16384]);
        let connection = log_server();
        for chunk in buf.chunks(lend_buffer.0.len()) {
            for (dest, src) in lend_buffer.0.iter_mut().zip(chunk) {
                *dest = *src;
            }
            lend(connection, LogLend::StandardError.into(), &lend_buffer.0, 0, chunk.len())
                .unwrap();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub const STDIN_BUF_SIZE: usize = unsupported_stdio::STDIN_BUF_SIZE;

pub fn is_ebadf(_err: &io::Error) -> bool {
    true
}

#[derive(Copy, Clone)]
pub struct PanicWriter {
    log: Connection,
}

impl io::Write for PanicWriter {
    fn write(&mut self, s: &[u8]) -> core::result::Result<usize, io::Error> {
        for c in s.chunks(size_of::<usize>() * 4) {
            try_scalar(self.log, LogScalar::AppendPanicMessage(&c).into()).ok();
        }
        Ok(s.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn panic_output() -> Option<impl io::Write> {
    let log = log_server();
    try_scalar(log, LogScalar::BeginPanic.into()).ok();
    Some(PanicWriter { log })
}
