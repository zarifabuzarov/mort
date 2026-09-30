use std::env;
use wasmtime::*;
use crate::state::HostState;
use crate::utils::{read_str_from_mem, write_bytes_to_mem};

pub fn register_env_api(linker: &mut Linker<HostState>) -> Result<()> {
    linker.func_wrap(
        "env",
        "host_get_env_len",
        |mut caller: Caller<'_, HostState>, key_ptr: i32, key_len: i32| -> u32 {
            let Ok(key) = read_str_from_mem(&mut caller, key_ptr, key_len) else {
                return 0;
            };

            env::var(&key).map(|v| v.len() as u32).unwrap_or(0)
        },
    )?;

    linker.func_wrap(
        "env",
        "host_get_env",
        |mut caller: Caller<'_, HostState>, key_ptr: i32, key_len: i32, out_ptr: i32, out_max_len: i32| -> u32 {
            let Ok(key) = read_str_from_mem(&mut caller, key_ptr, key_len) else {
                return 0;
            };

            if let Ok(val) = env::var(&key) {
                let bytes = val.as_bytes();
                let copy_len = bytes.len().min(out_max_len as usize);

                if write_bytes_to_mem(&mut caller, out_ptr, &bytes[..copy_len]).is_ok() {
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