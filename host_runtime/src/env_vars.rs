use std::env;
use wasmtime::*;
use crate::state::HostState;

pub fn register_env_api(linker: &mut Linker<HostState>) -> Result<()> {
    linker.func_wrap(
        "env",
        "host_get_env_len",
        |mut caller: Caller<'_, HostState>, key_ptr: u32, key_len: u32| -> u32 {
            let memory = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => return 0,
            };

            let data = memory.data(&caller);
            let start = key_ptr as usize;
            let end = start.saturating_add(key_len as usize);

            if end > data.len() {
                return 0;
            }

            let key = match std::str::from_utf8(&data[start..end]) {
                Ok(s) => s,
                Err(_) => return 0,
            };

            // ОТЛАДКА: смотрим, какой ключ запрашивает WASM
            println!("[Host Debug] Запрос длины env: '{}'", key);

            env::var(key).map(|v| v.len() as u32).unwrap_or(0)
        },
    )?;

    linker.func_wrap(
        "env",
        "host_get_env",
        |mut caller: Caller<'_, HostState>, key_ptr: u32, key_len: u32, out_ptr: u32, out_max_len: u32| -> u32 {
            let memory = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => return 0,
            };

            let data = memory.data(&caller);
            let k_start = key_ptr as usize;
            let k_end = k_start.saturating_add(key_len as usize);

            if k_end > data.len() {
                return 0;
            }

            let key = match std::str::from_utf8(&data[k_start..k_end]) {
                Ok(s) => s,
                Err(_) => return 0,
            };

            if let Ok(val) = env::var(key) {
                let bytes = val.as_bytes();
                let copy_len = bytes.len().min(out_max_len as usize);

                if memory.write(&mut caller, out_ptr as usize, &bytes[..copy_len]).is_ok() {
                    copy_len as u32
                } else {
                    0
                }
            } else {
                0
            }
        },
    )?;

    Ok(())
}