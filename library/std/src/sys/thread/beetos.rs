use core::arch::asm;

use crate::io;
use crate::num::NonZero;
use crate::os::beetos::ffi::{
    MemoryFlags, Syscall, ThreadId, blocking_scalar, create_thread, do_yield, join_thread,
    map_memory, update_memory_flags,
};
use crate::os::beetos::services::{TicktimerScalar, ticktimer_server};
use crate::thread::ThreadInit;
use crate::time::Duration;

pub struct Thread {
    tid: ThreadId,
}

pub const DEFAULT_MIN_STACK_SIZE: usize = 131072;
const MIN_STACK_SIZE: usize = 4096;
pub const GUARD_PAGE_SIZE: usize = 4096;

impl Thread {
    // unsafe: see thread::Builder::spawn_unchecked for safety requirements
    pub unsafe fn new(stack: usize, init: Box<ThreadInit>) -> io::Result<Thread> {
        let data = Box::into_raw(init);
        let mut stack_size = crate::cmp::max(stack, MIN_STACK_SIZE);

        if (stack_size & 4095) != 0 {
            stack_size = (stack_size + 4095) & !4095;
        }

        let stack_plus_guard_pages: &mut [u8] = unsafe {
            map_memory(
                None,
                None,
                GUARD_PAGE_SIZE + stack_size + GUARD_PAGE_SIZE,
                MemoryFlags::R | MemoryFlags::W | MemoryFlags::X,
            )
        }
        .map_err(|code| io::Error::from_raw_os_error(code as i32))?;

        unsafe {
            update_memory_flags(&mut stack_plus_guard_pages[0..GUARD_PAGE_SIZE], MemoryFlags::W)
                .map_err(|code| io::Error::from_raw_os_error(code as i32))?
        };

        unsafe {
            update_memory_flags(
                &mut stack_plus_guard_pages[(GUARD_PAGE_SIZE + stack_size)..],
                MemoryFlags::W,
            )
            .map_err(|code| io::Error::from_raw_os_error(code as i32))?
        };

        let guard_page_pre = stack_plus_guard_pages.as_ptr() as usize;
        let tid = create_thread(
            thread_start as *mut usize,
            &mut stack_plus_guard_pages[GUARD_PAGE_SIZE..(stack_size + GUARD_PAGE_SIZE)],
            data as usize,
            guard_page_pre,
            stack_size,
            0,
        )
        .map_err(|code| io::Error::from_raw_os_error(code as i32))?;

        #[inline(never)]
        fn rust_main_thread_not_inlined(init: Box<ThreadInit>) {
            let rust_start = init.init();
            rust_start();
        }

        extern "C" fn thread_start(
            data: *mut usize,
            guard_page_pre: usize,
            stack_size: usize,
        ) -> ! {
            let init = unsafe { Box::from_raw(data as *mut ThreadInit) };

            rust_main_thread_not_inlined(init);

            unsafe {
                crate::sys::thread_local::key::destroy_tls();
            }

            // Unmap stack memory and exit thread via kernel.
            // AArch64 ABI: x0-x5, x8-x9 for syscall args.
            let mapped_memory_base = guard_page_pre;
            let mapped_memory_length = GUARD_PAGE_SIZE + stack_size + GUARD_PAGE_SIZE;
            unsafe {
                asm!(
                    "svc #0",
                    // After UnmapMemory, branch to thread exit address.
                    // The kernel will clean up the thread on return from this.
                    "brk #0",
                    in("x0") Syscall::UnmapMemory as usize,
                    in("x1") mapped_memory_base,
                    in("x2") mapped_memory_length,
                    in("x3") 0usize,
                    in("x4") 0usize,
                    in("x5") 0usize,
                    in("x8") 0usize,
                    in("x9") 0usize,
                    options(nomem, nostack, noreturn)
                );
            }
        }

        Ok(Thread { tid })
    }

    pub fn join(self) {
        join_thread(self.tid).unwrap();
    }
}

pub fn available_parallelism() -> io::Result<NonZero<usize>> {
    Ok(unsafe { NonZero::new_unchecked(1) })
}

pub fn yield_now() {
    do_yield();
}

pub fn sleep(dur: Duration) {
    let mut millis = dur.as_millis();
    while millis > 0 {
        let sleep_duration = if millis > (usize::MAX as _) { usize::MAX } else { millis as usize };
        blocking_scalar(ticktimer_server(), TicktimerScalar::SleepMs(sleep_duration).into())
            .expect("failed to send message to ticktimer server");
        millis -= sleep_duration as u128;
    }
}
