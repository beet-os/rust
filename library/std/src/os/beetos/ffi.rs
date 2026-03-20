#![allow(dead_code)]
#![allow(unused_variables)]
#![stable(feature = "rust1", since = "1.0.0")]

#[path = "../unix/ffi/os_str.rs"]
mod os_str;

#[stable(feature = "rust1", since = "1.0.0")]
pub use self::os_str::{OsStrExt, OsStringExt};

mod definitions;
#[stable(feature = "rust1", since = "1.0.0")]
pub use definitions::*;

/// Issue a raw BeetOS/Xous syscall via `svc #0`.
///
/// AArch64 ABI: arguments in x0-x5, x8-x9 (8 registers), returns in same.
#[inline(always)]
fn raw_syscall(args: [usize; 8]) -> [usize; 8] {
    let r0: usize;
    let r1: usize;
    let r2: usize;
    let r3: usize;
    let r4: usize;
    let r5: usize;
    let r8: usize;
    let r9: usize;

    unsafe {
        core::arch::asm!(
            "svc #0",
            inlateout("x0") args[0] => r0,
            inlateout("x1") args[1] => r1,
            inlateout("x2") args[2] => r2,
            inlateout("x3") args[3] => r3,
            inlateout("x4") args[4] => r4,
            inlateout("x5") args[5] => r5,
            inlateout("x8") args[6] => r8,
            inlateout("x9") args[7] => r9,
            // Clobber caller-saved registers not used for args/results
            lateout("x6") _,
            lateout("x7") _,
            lateout("x10") _,
            lateout("x11") _,
            lateout("x12") _,
            lateout("x13") _,
            lateout("x14") _,
            lateout("x15") _,
            lateout("x16") _,
            lateout("x17") _,
        );
    }

    [r0, r1, r2, r3, r4, r5, r8, r9]
}

fn lend_mut_impl(
    connection: Connection,
    opcode: usize,
    data: &mut [u8],
    arg1: usize,
    arg2: usize,
    blocking: bool,
) -> Result<(usize, usize), Error> {
    let syscall_nr = if blocking { Syscall::SendMessage } else { Syscall::TrySendMessage } as usize;
    let cid: usize = connection.try_into().unwrap();
    let invoke = InvokeType::LendMut as usize;

    let ret = raw_syscall([
        syscall_nr,
        cid,
        invoke,
        opcode,
        data.as_mut_ptr() as usize,
        data.len(),
        arg1,
        arg2,
    ]);

    if ret[0] == SyscallResult::MemoryReturned as usize {
        Ok((ret[1], ret[2]))
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}

pub(crate) fn lend_mut(
    connection: Connection,
    opcode: usize,
    data: &mut [u8],
    arg1: usize,
    arg2: usize,
) -> Result<(usize, usize), Error> {
    lend_mut_impl(connection, opcode, data, arg1, arg2, true)
}

pub(crate) fn try_lend_mut(
    connection: Connection,
    opcode: usize,
    data: &mut [u8],
    arg1: usize,
    arg2: usize,
) -> Result<(usize, usize), Error> {
    lend_mut_impl(connection, opcode, data, arg1, arg2, false)
}

fn lend_impl(
    connection: Connection,
    opcode: usize,
    data: &[u8],
    arg1: usize,
    arg2: usize,
    blocking: bool,
) -> Result<(usize, usize), Error> {
    let syscall_nr = if blocking { Syscall::SendMessage } else { Syscall::TrySendMessage } as usize;
    let cid: usize = connection.try_into().unwrap();
    let invoke = InvokeType::Lend as usize;

    let ret = raw_syscall([
        syscall_nr,
        cid,
        invoke,
        opcode,
        data.as_ptr() as usize,
        data.len(),
        arg1,
        arg2,
    ]);

    if ret[0] == SyscallResult::MemoryReturned as usize {
        Ok((ret[1], ret[2]))
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}

pub(crate) fn lend(
    connection: Connection,
    opcode: usize,
    data: &[u8],
    arg1: usize,
    arg2: usize,
) -> Result<(usize, usize), Error> {
    lend_impl(connection, opcode, data, arg1, arg2, true)
}

pub(crate) fn try_lend(
    connection: Connection,
    opcode: usize,
    data: &[u8],
    arg1: usize,
    arg2: usize,
) -> Result<(usize, usize), Error> {
    lend_impl(connection, opcode, data, arg1, arg2, false)
}

fn scalar_impl(connection: Connection, args: [usize; 5], blocking: bool) -> Result<(), Error> {
    let syscall_nr = if blocking { Syscall::SendMessage } else { Syscall::TrySendMessage } as usize;
    let cid: usize = connection.try_into().unwrap();
    let invoke = InvokeType::Scalar as usize;

    let ret = raw_syscall([
        syscall_nr,
        cid,
        invoke,
        args[0],
        args[1],
        args[2],
        args[3],
        args[4],
    ]);

    if ret[0] == SyscallResult::Ok as usize {
        Ok(())
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}

pub(crate) fn scalar(connection: Connection, args: [usize; 5]) -> Result<(), Error> {
    scalar_impl(connection, args, true)
}

pub(crate) fn try_scalar(connection: Connection, args: [usize; 5]) -> Result<(), Error> {
    scalar_impl(connection, args, false)
}

fn blocking_scalar_impl(
    connection: Connection,
    args: [usize; 5],
    blocking: bool,
) -> Result<[usize; 5], Error> {
    let syscall_nr = if blocking { Syscall::SendMessage } else { Syscall::TrySendMessage } as usize;
    let cid: usize = connection.try_into().unwrap();
    let invoke = InvokeType::BlockingScalar as usize;

    let ret = raw_syscall([
        syscall_nr,
        cid,
        invoke,
        args[0],
        args[1],
        args[2],
        args[3],
        args[4],
    ]);

    if ret[0] == SyscallResult::Scalar1 as usize {
        Ok([ret[1], 0, 0, 0, 0])
    } else if ret[0] == SyscallResult::Scalar2 as usize {
        Ok([ret[1], ret[2], 0, 0, 0])
    } else if ret[0] == SyscallResult::Scalar5 as usize {
        Ok([ret[1], ret[2], ret[3], ret[4], ret[5]])
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}

pub(crate) fn blocking_scalar(
    connection: Connection,
    args: [usize; 5],
) -> Result<[usize; 5], Error> {
    blocking_scalar_impl(connection, args, true)
}

pub(crate) fn try_blocking_scalar(
    connection: Connection,
    args: [usize; 5],
) -> Result<[usize; 5], Error> {
    blocking_scalar_impl(connection, args, false)
}

fn connect_impl(address: ServerAddress, blocking: bool) -> Result<Connection, Error> {
    let syscall_nr = if blocking { Syscall::Connect } else { Syscall::TryConnect } as usize;
    let address: [u32; 4] = address.into();

    let ret = raw_syscall([
        syscall_nr,
        address[0].try_into().unwrap(),
        address[1].try_into().unwrap(),
        address[2].try_into().unwrap(),
        address[3].try_into().unwrap(),
        0,
        0,
        0,
    ]);

    if ret[0] == SyscallResult::ConnectionId as usize {
        Ok(ret[1].try_into().unwrap())
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}

pub(crate) fn connect(address: ServerAddress) -> Result<Connection, Error> {
    connect_impl(address, true)
}

pub(crate) fn try_connect(address: ServerAddress) -> Result<Option<Connection>, Error> {
    match connect_impl(address, false) {
        Ok(conn) => Ok(Some(conn)),
        Err(Error::ServerNotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Terminates the current process and returns the specified code to the parent process.
pub(crate) fn exit(return_code: u32) -> ! {
    raw_syscall([
        Syscall::TerminateProcess as usize,
        return_code as usize,
        0,
        0,
        0,
        0,
        0,
        0,
    ]);
    unreachable!();
}

/// Suspends the current thread and allows another thread to run.
pub(crate) fn do_yield() {
    raw_syscall([
        Syscall::Yield as usize,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ]);
}

/// Allocates memory from the system.
///
/// # Safety
///
/// This function is safe unless a virtual address is specified. In that case,
/// the kernel will return an alias to the existing range.
pub(crate) unsafe fn map_memory<T>(
    phys: Option<core::ptr::NonNull<T>>,
    virt: Option<core::ptr::NonNull<T>>,
    count: usize,
    flags: MemoryFlags,
) -> Result<&'static mut [T], Error> {
    let ret = raw_syscall([
        Syscall::MapMemory as usize,
        phys.map(|p| p.as_ptr() as usize).unwrap_or_default(),
        virt.map(|p| p.as_ptr() as usize).unwrap_or_default(),
        count * size_of::<T>(),
        flags.bits(),
        0,
        0,
        0,
    ]);

    if ret[0] == SyscallResult::MemoryRange as usize {
        let start = core::ptr::with_exposed_provenance_mut::<T>(ret[1]);
        let len = ret[2] / size_of::<T>();
        Ok(unsafe { core::slice::from_raw_parts_mut(start, len) })
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}

/// Destroys the given memory, returning it to the kernel.
///
/// # Safety
///
/// The memory pointed to by `range` should not be used after this
/// function returns.
pub(crate) unsafe fn unmap_memory<T>(range: *mut [T]) -> Result<(), Error> {
    let ret = raw_syscall([
        Syscall::UnmapMemory as usize,
        range.as_mut_ptr() as usize,
        range.len() * size_of::<T>(),
        0,
        0,
        0,
        0,
        0,
    ]);

    if ret[0] == SyscallResult::Ok as usize {
        Ok(())
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}

/// Adjusts the memory flags for the given range.
///
/// # Safety
///
/// The memory pointed to by `range` may become inaccessible or have its
/// mutability removed.
pub(crate) unsafe fn update_memory_flags<T>(
    range: *mut [T],
    new_flags: MemoryFlags,
) -> Result<(), Error> {
    let ret = raw_syscall([
        Syscall::UpdateMemoryFlags as usize,
        range.as_mut_ptr() as usize,
        range.len() * size_of::<T>(),
        new_flags.bits(),
        0, // Process ID is currently None
        0,
        0,
        0,
    ]);

    if ret[0] == SyscallResult::Ok as usize {
        Ok(())
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}

/// Creates a thread with a given stack and up to four arguments.
pub(crate) fn create_thread(
    start: *mut usize,
    stack: *mut [u8],
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> Result<ThreadId, Error> {
    let ret = raw_syscall([
        Syscall::CreateThread as usize,
        start as usize,
        stack.as_mut_ptr() as usize,
        stack.len(),
        arg0,
        arg1,
        arg2,
        arg3,
    ]);

    if ret[0] == SyscallResult::ThreadId as usize {
        Ok(ret[1].into())
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}

/// Waits for the given thread to terminate and returns the exit code.
pub(crate) fn join_thread(thread_id: ThreadId) -> Result<usize, Error> {
    let ret = raw_syscall([
        Syscall::JoinThread as usize,
        thread_id.into(),
        0,
        0,
        0,
        0,
        0,
        0,
    ]);

    if ret[0] == SyscallResult::Scalar1 as usize
        || ret[0] == SyscallResult::Scalar2 as usize
        || ret[0] == SyscallResult::Scalar5 as usize
    {
        Ok(ret[1])
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}

/// Gets the current thread's ID.
pub(crate) fn thread_id() -> Result<ThreadId, Error> {
    let ret = raw_syscall([
        Syscall::GetThreadId as usize,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ]);

    if ret[0] == SyscallResult::ThreadId as usize {
        Ok(ret[1].into())
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}

/// Adjusts the given `knob` limit to match the new value `new`.
pub(crate) fn adjust_limit(knob: Limits, current: usize, new: usize) -> Result<usize, Error> {
    let ret = raw_syscall([
        Syscall::AdjustProcessLimit as usize,
        knob as usize,
        current,
        new,
        0,
        0,
        0,
        0,
    ]);

    if ret[0] == SyscallResult::Scalar2 as usize && ret[1] == knob as usize {
        Ok(ret[2])
    } else if ret[0] == SyscallResult::Scalar5 as usize && ret[1] == knob as usize {
        Ok(ret[1])
    } else if ret[0] == SyscallResult::Error as usize {
        Err(ret[1].into())
    } else {
        Err(Error::InternalError)
    }
}
