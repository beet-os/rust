#![forbid(unsafe_op_in_unsafe_fn)]

pub mod os;

#[path = "../unsupported/common.rs"]
mod common;
pub use common::*;

#[cfg(not(test))]
mod c_compat {
    use crate::os::beetos::ffi::exit;

    unsafe extern "C" {
        fn main() -> u32;
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn abort() {
        exit(1);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn _start() {
        exit(unsafe { main() });
    }
}
