use wasmtime::*;
use std::io::{self, BufRead};

pub fn register_console_api(linker: &mut Linker<()>) -> Result<()> {
    
    linker.func_wrap("env", "host_print", |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
        if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
            let data = mem.data(&caller);
            let start = ptr as usize;
            let end = start + len as usize;

            // Вычитываем байты из ОЗУ WASM и переводим в UTF-8 строку
            if let Ok(msg) = std::str::from_utf8(&data[start..end]) {
                println!("[Host Print]: {msg}");
            }
        }
    })?;

    // Чтение строки (Stdin Input)
    linker.func_wrap(
        "env",
        "host_read_line",
        |mut caller: Caller<'_, ()>, out_ptr: u32, out_max_len: u32| -> u32 {
            let mut input = String::new();
            let stdin = io::stdin();
            if stdin.lock().read_line(&mut input).is_ok() {
                let trimmed = input.trim_end(); // Убираем \n в конце
                let bytes = trimmed.as_bytes();
                let write_len = std::cmp::min(bytes.len(), out_max_len as usize);

                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => return 0,
                };

                let memory_mut = memory.data_mut(&mut caller);
                let start = out_ptr as usize;
                let end = start + write_len;

                memory_mut[start..end].copy_from_slice(&bytes[..write_len]);
                write_len as u32
            } else {
                0
            }
        },
    )?;

    Ok(())
}