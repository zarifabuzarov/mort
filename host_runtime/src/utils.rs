use std::str::Utf8Error;
use thiserror::Error;
use wasmtime::Caller;

use crate::state::HostState;

pub const MAX_TRANSFER_SIZE: usize = 8 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum MemError {
    #[error("Export 'memory' not found or invalid")]
    MemoryNotFound,
    #[error("Pointer or length conversion failed / overflow")]
    InvalidAddress,
    #[error("Requested size exceeds safety limit")]
    SizeExceedsLimit,
    #[error("Memory access out of bounds")]
    OutOfBounds,
    #[error("Invalid UTF-8 sequence: {0}")]
    InvalidUtf8(#[from] Utf8Error),
}

fn get_memory(caller: &mut Caller<'_, HostState>) -> Result<wasmtime::Memory, MemError> {
    caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or(MemError::MemoryNotFound)
}

fn validate_bounds(ptr: i32, len: usize) -> Result<(usize, usize), MemError> {
    if len > MAX_TRANSFER_SIZE {
        return Err(MemError::SizeExceedsLimit);
    }
    let start = ptr as u32 as usize;
    let end = start.checked_add(len).ok_or(MemError::InvalidAddress)?;
    Ok((start, end))
}

pub fn with_mem_slice<F, R>(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
    f: F,
) -> Result<R, MemError>
where
    F: FnOnce(&[u8]) -> R,
{
    let (start, end) = validate_bounds(ptr, len as u32 as usize)?;
    let memory = get_memory(caller)?;

    let slice = memory
        .data(caller)
        .get(start..end)
        .ok_or(MemError::OutOfBounds)?;

    Ok(f(slice))
}

pub fn with_mem_slice_mut<F, R>(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
    f: F,
) -> Result<R, MemError>
where
    F: FnOnce(&mut [u8]) -> R,
{
    let (start, end) = validate_bounds(ptr, len as u32 as usize)?;
    let memory = get_memory(caller)?;

    let slice = memory
        .data_mut(caller)
        .get_mut(start..end)
        .ok_or(MemError::OutOfBounds)?;

    Ok(f(slice))
}

#[allow(dead_code)]
pub fn with_str_from_mem<F, R>(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
    f: F,
) -> Result<R, MemError>
where
    F: FnOnce(&str) -> R,
{
    with_mem_slice(caller, ptr, len, |bytes| {
        let s = std::str::from_utf8(bytes)?;
        Ok(f(s))
    })?
}

pub fn read_bytes_from_mem(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
) -> Result<Vec<u8>, MemError> {
    with_mem_slice(caller, ptr, len, |slice| slice.to_vec())
}

pub fn read_str_from_mem(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
) -> Result<String, MemError> {
    with_str_from_mem(caller, ptr, len, |s| s.to_string())
}

pub fn write_bytes_to_mem(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    bytes: &[u8],
) -> Result<(), MemError> {
    if bytes.len() > MAX_TRANSFER_SIZE {
        return Err(MemError::SizeExceedsLimit);
    }

    with_mem_slice_mut(caller, ptr, bytes.len() as i32, |slice| {
        slice.copy_from_slice(bytes);
    })
}

#[allow(dead_code)]
pub fn write_str_to_mem(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    s: &str,
) -> Result<(), MemError> {
    write_bytes_to_mem(caller, ptr, s.as_bytes())
}

