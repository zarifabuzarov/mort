use wasmtime::*;
use sysinfo::System;
use std::process;
use crate::state::HostState;
use crate::utils::{write_bytes_to_mem};

pub fn register_system_api(linker: &mut Linker<HostState>) -> anyhow::Result<()> {
    linker.func_wrap("env", "host_get_pid", |_caller: Caller<'_, HostState>| -> u32 {
        process::id()
    })?;

    linker.func_wrap(
            "env",
            "host_exit",
            |_caller: Caller<'_, HostState>, code: i32| -> Result<(), Error> {
                println!("[Host Process] Выход из WASM с кодом: {}", code);

                if code == 0 {
                    // Мягко прерываем исполнение WASM-модуля (не роняя сам хост!)
                    Err(Error::msg("WASM_EXIT_SUCCESS"))
                } else {
                    Err(Error::msg(format!("WASM_EXIT_FAILURE: {code}")))
                }
            }
        )?;

    linker.func_wrap("env", "host_cpu_count", |_caller: Caller<'_, HostState>| -> u32 {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1)
    })?;

    linker.func_wrap("env", "host_total_memory", |_caller: Caller<'_, HostState>| -> u64 {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.total_memory() // байты
    })?;

    linker.func_wrap("env", "host_free_memory", |_caller: Caller<'_, HostState>| -> u64 {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.available_memory() // байты
    })?;

    // Получение длины всех аргументов, разделенных '\0'
    linker.func_wrap("env", "host_get_args_len", |_caller: Caller<'_, HostState>| -> u32 {
        let args: Vec<String> = std::env::args().collect();
        let joined = args.join("\0");
        joined.len() as u32
    })?;

    // Запись аргументов в память WASM
    linker.func_wrap(
        "env",
        "host_get_args",
        |mut caller: Caller<'_, HostState>, out_ptr: i32, max_len: i32| -> u32 {
            let args: Vec<String> = std::env::args().collect();
            let joined = args.join("\0");
            let bytes = joined.as_bytes();

            let to_write = bytes.len().min(max_len as usize);
            if write_bytes_to_mem(&mut caller, out_ptr, &bytes[..to_write]).is_ok() {
                to_write as u32
            } else {
                0
            }
        },
    )?;

    Ok(())
}