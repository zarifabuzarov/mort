use wasmtime::*;
use std::io::{self, BufRead, Write};
use crate::state::HostState;
use crate::utils::{read_str_from_mem, write_bytes_to_mem};

pub fn register_console_api(linker: &mut Linker<HostState>) -> Result<()> {

    // Стандартный вывод (Stdout)
    linker.func_wrap(
        "env",
        "host_print",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
            if let Ok(msg) = read_str_from_mem(&mut caller, ptr, len) {
                print!("{msg}");
                let _ = io::stdout().flush();
            }
        }
    )?;

    // Вывод ошибок (Stderr)
    linker.func_wrap(
        "env",
        "host_print_err",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
            if let Ok(msg) = read_str_from_mem(&mut caller, ptr, len) {
                eprint!("{msg}");
                let _ = io::stderr().flush();
            }
        }
    )?;

    // Чтение строки (Stdin Input)
    linker.func_wrap(
        "env",
        "host_read_line",
        |mut caller: Caller<'_, HostState>, out_ptr: i32, max_len: i32| -> i32 {
            let mut input = String::new();
            let stdin = io::stdin();

            if stdin.lock().read_line(&mut input).is_ok() {
                let trimmed = input.trim_end(); // Убираем \n или \r\n в конце
                let bytes = trimmed.as_bytes();

                if bytes.len() > max_len as usize {
                    return -1; // Введенная строка не влезает в буфер WASM
                }

                match write_bytes_to_mem(&mut caller, out_ptr, bytes) {
                    Ok(_) => bytes.len() as i32,
                    Err(_) => -1,
                }
            } else {
                -1
            }
        },
    )?;

    Ok(())
}