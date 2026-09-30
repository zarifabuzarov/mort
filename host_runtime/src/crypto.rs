use rand::RngCore;
use ring::digest::{digest, SHA256, SHA512};
use wasmtime::*;
use crate::state::HostState;
use crate::utils::{with_mem_slice, with_mem_slice_mut};

pub fn register_crypto_api(linker: &mut Linker<HostState>) -> Result<()> {
    // 1. Генерация random u32
    linker.func_wrap("env", "host_random_u32", |_caller: Caller<'_, HostState>| -> u32 {
        let mut rng = rand::thread_rng();
        rng.next_u32()
    })?;

    // 2. Заполнение буфера случайными байтами через Zero-Copy слайс
    linker.func_wrap(
        "env",
        "host_random_bytes",
        |mut caller: Caller<'_, HostState>, out_ptr: i32, len: i32| -> i32 {
            let res = with_mem_slice_mut(&mut caller, out_ptr, len, |buf| {
                rand::thread_rng().fill_bytes(buf);
            });

            if res.is_ok() { 0 } else { -1 }
        },
    )?;

    // 3. Хеширование (SHA-256 / SHA-512) без лишней аллокации промежуточного Vec
    linker.func_wrap(
        "env",
        "host_hash",
        |mut caller: Caller<'_, HostState>,
         alg: i32,
         data_ptr: i32,
         data_len: i32,
         out_ptr: i32,
         out_max_len: i32| -> i32 {
            let expected_len = match alg {
                0 => 32, // SHA-256
                1 => 64, // SHA-512
                _ => return -2,
            };

            if (out_max_len as usize) < expected_len {
                return -3;
            }

            // Читаем вход прямо из WASM-памяти по ссылке
            let hash_bytes = match with_mem_slice(&mut caller, data_ptr, data_len, |input| {
                match alg {
                    0 => digest(&SHA256, input),
                    1 => digest(&SHA512, input),
                    _ => unreachable!(),
                }
            }) {
                Ok(h) => h,
                Err(_) => return -1,
            };

            // Пишем результат напрямую в выходной буфер WASM
            let write_res = with_mem_slice_mut(&mut caller, out_ptr, expected_len as i32, |out| {
                out.copy_from_slice(hash_bytes.as_ref());
            });

            if write_res.is_ok() {
                expected_len as i32
            } else {
                -1
            }
        },
    )?;

    Ok(())
}