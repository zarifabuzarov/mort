use std::env;
use wasmtime::*;
use crate::state::HostState;

pub fn register_env_api(linker: &mut Linker<HostState>) -> Result<()> {
    // Получение переменной окружения по имени
    // host_get_env(key_ptr, key_len, out_buf_ptr) -> u32 (длина записанного значения)
    linker.func_wrap(
        "env",
        "host_get_env",
        |mut caller: Caller<'_, HostState>, key_ptr: u32, key_len: u32, out_ptr: u32| -> u32 {
            let memory = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => return 0,
            };

            let data = memory.data(&caller);
            let key = match std::str::from_utf8(&data[key_ptr as usize..(key_ptr + key_len) as usize]) {
                Ok(s) => s,
                Err(_) => return 0,
            };

            if let Ok(val) = env::var(key) {
                let bytes = val.as_bytes();
                let memory_mut = memory.data_mut(&mut caller);
                
                let start = out_ptr as usize;
                let end = start + bytes.len();
                memory_mut[start..end].copy_from_slice(bytes);

                bytes.len() as u32
            } else {
                0
            }
        },
    )?;

    Ok(())
}