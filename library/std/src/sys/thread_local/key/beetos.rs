//! Thread Local Storage for BeetOS (AArch64)
//!
//! Uses TPIDR_EL0 as the thread-local base pointer (equivalent to RISC-V's `tp`).
//! Currently limited to 1023 TLS entries stored in a single page per thread.

use core::arch::asm;

use crate::alloc::System;
use crate::mem::ManuallyDrop;
use crate::os::beetos::ffi::{MemoryFlags, map_memory, unmap_memory};
use crate::ptr;
use crate::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use crate::sync::atomic::{Atomic, AtomicPtr, AtomicUsize};

pub type Key = usize;
pub type Dtor = unsafe extern "C" fn(*mut u8);

const TLS_MEMORY_SIZE: usize = 16 * 1024; // BeetOS uses 16KB pages (AArch64); must be page-aligned

#[cfg(not(test))]
#[unsafe(export_name = "_ZN16__rust_internals3std3sys6beetos16thread_local_key13TLS_KEY_INDEXE")]
static TLS_KEY_INDEX: Atomic<usize> = AtomicUsize::new(1);

#[cfg(not(test))]
#[unsafe(export_name = "_ZN16__rust_internals3std3sys6beetos16thread_local_key9DTORSE")]
static DTORS: Atomic<*mut Node> = AtomicPtr::new(ptr::null_mut());

#[cfg(test)]
unsafe extern "Rust" {
    #[link_name = "_ZN16__rust_internals3std3sys6beetos16thread_local_key13TLS_KEY_INDEXE"]
    static TLS_KEY_INDEX: Atomic<usize>;

    #[link_name = "_ZN16__rust_internals3std3sys6beetos16thread_local_key9DTORSE"]
    static DTORS: Atomic<*mut Node>;
}

fn tls_ptr_addr() -> *mut *mut u8 {
    let tp: usize;
    unsafe {
        asm!(
            "mrs {}, TPIDR_EL0",
            out(reg) tp,
        );
    }
    core::ptr::with_exposed_provenance_mut::<*mut u8>(tp)
}

fn tls_table() -> &'static mut [*mut u8] {
    let tp = tls_ptr_addr();

    if !tp.is_null() {
        return unsafe {
            core::slice::from_raw_parts_mut(tp, TLS_MEMORY_SIZE / size_of::<*mut u8>())
        };
    }

    let tp = unsafe {
        map_memory(
            None,
            None,
            TLS_MEMORY_SIZE / size_of::<*mut u8>(),
            MemoryFlags::R | MemoryFlags::W,
        )
        .expect("Unable to allocate memory for thread local storage")
    };

    // BeetOS: zero the TLS table explicitly — MapMemory does not guarantee zero-initialized pages.
    for val in tp.iter_mut() {
        *val = core::ptr::null_mut();
    }

    unsafe {
        asm!(
            "msr TPIDR_EL0, {}",
            in(reg) tp.as_mut_ptr() as usize,
        );
    }
    tp
}

#[inline]
pub fn create(dtor: Option<Dtor>) -> Key {
    #[allow(unused_unsafe)]
    let key = unsafe { TLS_KEY_INDEX.fetch_add(1, Relaxed) };
    if let Some(f) = dtor {
        unsafe { register_dtor(key, f) };
    }
    key
}

#[inline]
pub unsafe fn set(key: Key, value: *mut u8) {
    assert!((key < 1022) && (key >= 1));
    let table = tls_table();
    table[key] = value;
}

#[inline]
pub unsafe fn get(key: Key) -> *mut u8 {
    assert!((key < 1022) && (key >= 1));
    tls_table()[key]
}

#[inline]
pub unsafe fn destroy(_key: Key) {
    // Just leak the key.
}

struct Node {
    dtor: Dtor,
    key: Key,
    next: *mut Node,
}

unsafe fn register_dtor(key: Key, dtor: Dtor) {
    let mut node =
        ManuallyDrop::new(Box::new_in(Node { key, dtor, next: ptr::null_mut() }, System));

    #[allow(unused_unsafe)]
    let mut head = unsafe { DTORS.load(Acquire) };
    loop {
        node.next = head;
        #[allow(unused_unsafe)]
        match unsafe { DTORS.compare_exchange(head, &mut **node, Release, Acquire) } {
            Ok(_) => return,
            Err(cur) => head = cur,
        }
    }
}

pub unsafe fn destroy_tls() {
    let tp = tls_ptr_addr();

    if tp.is_null() {
        return;
    }

    unsafe { run_dtors() };

    unsafe {
        unmap_memory(core::slice::from_raw_parts_mut(tp, TLS_MEMORY_SIZE / size_of::<usize>()))
            .unwrap()
    };
}

#[inline(never)]
unsafe fn run_dtors() {
    let mut any_run = true;

    for _ in 0..5 {
        if !any_run {
            break;
        }
        any_run = false;
        #[allow(unused_unsafe)]
        let mut cur = unsafe { DTORS.load(Acquire) };
        while !cur.is_null() {
            let ptr = unsafe { get((*cur).key) };

            if !ptr.is_null() {
                unsafe { set((*cur).key, ptr::null_mut()) };
                unsafe { ((*cur).dtor)(ptr as *mut _) };
                any_run = true;
            }

            unsafe { cur = (*cur).next };
        }
    }

    crate::rt::thread_cleanup();
}
