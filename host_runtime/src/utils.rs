use wasmtime::Caller;

use crate::state::SharedState;
use crate::error::{MOError, MOResult};

pub const MAX_TRANSFER_SIZE: usize = 8 * 1024 * 1024;

// #[derive(Debug, Error)]
// pub enum MemError {
//     #[error("Export 'memory' not found or invalid")]
//     MemoryNotFound,
//     #[error("Pointer or length conversion failed / overflow")]
//     InvalidAddress,
//     #[error("Requested size exceeds safety limit")]
//     SizeExceedsLimit,
//     #[error("Memory access out of bounds")]
//     OutOfBounds,
//     #[error("Invalid UTF-8 sequence: {0}")]
//     InvalidUtf8(#[from] Utf8Error),
// }

fn get_memory(caller: &mut Caller<'_, SharedState>) -> MOResult<wasmtime::Memory> {
    caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or(MOError::MemoryNotFound)
}

fn validate_bounds(ptr: i32, len: usize) -> MOResult<(usize, usize)> {
    if len > MAX_TRANSFER_SIZE {
        return Err(MOError::SizeExceedsLimit {
            size: len,
            max: MAX_TRANSFER_SIZE,
        });
    }
    let start = ptr as u32 as usize;
    let end = start.checked_add(len).ok_or(MOError::InvalidAddress)?;
    Ok((start, end))
}

pub fn with_mem_slice<F, R>(
    caller: &mut Caller<'_, SharedState>,
    ptr: i32,
    len: i32,
    f: F,
) -> MOResult<R>
where
    F: FnOnce(&[u8]) -> R,
{
    let (start, end) = validate_bounds(ptr, len as u32 as usize)?;
    let memory = get_memory(caller)?;

    let slice = memory
        .data(caller)
        .get(start..end)
        .ok_or(MOError::OutOfBounds { start, end })?;

    Ok(f(slice))
}

pub fn with_mem_slice_mut<F, R>(
    caller: &mut Caller<'_, SharedState>,
    ptr: i32,
    len: i32,
    f: F,
) -> MOResult<R>
where
    F: FnOnce(&mut [u8]) -> R,
{
    let (start, end) = validate_bounds(ptr, len as u32 as usize)?;
    let memory = get_memory(caller)?;

    let slice = memory
        .data_mut(caller)
        .get_mut(start..end)
        .ok_or(MOError::OutOfBounds { start, end })?;

    Ok(f(slice))
}

#[allow(dead_code)]
pub fn with_str_from_mem<F, R>(
    caller: &mut Caller<'_, SharedState>,
    ptr: i32,
    len: i32,
    f: F,
) -> MOResult<R>
where
    F: FnOnce(&str) -> R,
{
    with_mem_slice(caller, ptr, len, |bytes| {
        std::str::from_utf8(bytes)
            .map(f)
            .map_err(MOError::from)
    })?
}

pub fn read_bytes_from_mem(
    caller: &mut Caller<'_, SharedState>,
    ptr: i32,
    len: i32,
) -> MOResult<Vec<u8>> {
    with_mem_slice(caller, ptr, len, |slice| slice.to_vec())
}

pub fn read_str_from_mem(
    caller: &mut Caller<'_, SharedState>,
    ptr: i32,
    len: i32,
) -> MOResult<String> {
    with_str_from_mem(caller, ptr, len, |s| s.to_string())
}

pub fn write_bytes_to_mem(
    caller: &mut Caller<'_, SharedState>,
    ptr: i32,
    bytes: &[u8],
) -> MOResult<()> {
    if bytes.len() > MAX_TRANSFER_SIZE {
        return Err(MOError::SizeExceedsLimit {
            size: bytes.len(),
            max: MAX_TRANSFER_SIZE,
        });
    }

    with_mem_slice_mut(caller, ptr, bytes.len() as i32, |slice| {
        slice.copy_from_slice(bytes);
    })
}

#[allow(dead_code)]
pub fn write_str_to_mem(
    caller: &mut Caller<'_, SharedState>,
    ptr: i32,
    s: &str,
) -> MOResult<()> {
    write_bytes_to_mem(caller, ptr, s.as_bytes())
}

