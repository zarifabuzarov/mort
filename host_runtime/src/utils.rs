use std::str::Utf8Error;
use thiserror::Error;
use wasmtime::Caller;

use crate::state::HostState;

/// Максимальный размер передаваемых данных (8 МБ) для защиты от DoS
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

/// Приватный хелпер для получения экспортированной памяти WASM
fn get_memory(caller: &mut Caller<'_, HostState>) -> Result<wasmtime::Memory, MemError> {
    caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or(MemError::MemoryNotFound)
}

// Low-level: Zero-Copy доступ к байтам
// ВНИМАНИЕ: Не вызывайте Wasm-функции внутри замыкания `f`!
pub fn with_mem_slice<F, R>(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
    f: F,
) -> Result<R, MemError>
where
    F: FnOnce(&[u8]) -> R,
{
    let start = ptr as u32 as usize;
    let len = len as u32 as usize;

    if len > MAX_TRANSFER_SIZE {
        return Err(MemError::SizeExceedsLimit);
    }

    let end = start.checked_add(len).ok_or(MemError::InvalidAddress)?;
    let memory = get_memory(caller)?;

    let slice = memory
        .data(caller)
        .get(start..end)
        .ok_or(MemError::OutOfBounds)?;

    Ok(f(slice))
}

// Zero-Copy ЗАПИСЬ / МОДИФИКАЦИЯ прямо в памяти WASM
pub fn with_mem_slice_mut<F, R>(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
    f: F,
) -> Result<R, MemError>
where
    F: FnOnce(&mut [u8]) -> R, // Обрати внимание: &mut [u8]
{
    let start = ptr as u32 as usize;
    let len = len as u32 as usize;

    if len > MAX_TRANSFER_SIZE {
        return Err(MemError::SizeExceedsLimit);
    }

    let end = start.checked_add(len).ok_or(MemError::InvalidAddress)?;
    let memory = get_memory(caller)?;

    // Берем МУТАБЕЛЬНЫЙ срез памяти WASM
    let slice = memory
        .data_mut(caller)
        .get_mut(start..end)
        .ok_or(MemError::OutOfBounds)?;

    // Передаем мутабельную ссылку в замыкание для прямой записи/изменения
    Ok(f(slice))
}

// Low-level: Zero-Copy доступ к строке
pub fn with_str_from_mem<F, R>(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
    f: F,
) -> Result<R, MemError>
where
    F: FnOnce(&str) -> R,
{
    // Оборачиваем внутренний Result и переднимаем через ?
    with_mem_slice(caller, ptr, len, |bytes| {
        let s = std::str::from_utf8(bytes)?;
        Ok(f(s))
    })?
}

// High-level: Чтение байтов (копирование в Vec<u8>)
pub fn read_bytes_from_mem(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
) -> Result<Vec<u8>, MemError> {
    with_mem_slice(caller, ptr, len, |slice| slice.to_vec())
}

// High-level: Чтение строки (безопасно, с аллокацией String)
pub fn read_str_from_mem(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
) -> Result<String, MemError> {
    let bytes = read_bytes_from_mem(caller, ptr, len)?;
    String::from_utf8(bytes).map_err(|e| MemError::InvalidUtf8(e.utf8_error()))
}

// High-level: Запись байтов в память WASM
pub fn write_bytes_to_mem(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    bytes: &[u8],
) -> Result<(), MemError> {
    if bytes.len() > MAX_TRANSFER_SIZE {
        return Err(MemError::SizeExceedsLimit);
    }

    let start = ptr as u32 as usize;
    let end = start.checked_add(bytes.len()).ok_or(MemError::InvalidAddress)?;

    let memory = get_memory(caller)?;

    let slice = memory
        .data_mut(caller)
        .get_mut(start..end)
        .ok_or(MemError::OutOfBounds)?;

    slice.copy_from_slice(bytes);
    Ok(())
}

// High-level: Запись строки в память WASM
pub fn write_str_to_mem(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    s: &str,
) -> Result<(), MemError> {
    write_bytes_to_mem(caller, ptr, s.as_bytes())
}