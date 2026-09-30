use rand::RngCore;
use ring::digest::{digest, SHA256, SHA512};
use wasmtime::*;
use crate::state::HostState;

pub fn register_crypto_api(linker: &mut Linker<HostState>) -> Result<()> {
    // 1. Генерация random u32
    linker.func_wrap("env", "host_random_u32", |_caller: Caller<'_, HostState>| -> u32 {
        let mut rng = rand::thread_rng();
        rng.next_u32()
    })?;

    // 2. Заполнение произвольного буфера случайными байтами
    linker.func_wrap(
        "env",
        "host_random_bytes",
        |mut caller: Caller<'_, HostState>, out_ptr: u32, len: u32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => return -1,
            };

            let mut buf = vec![0u8; len as usize];
            rand::thread_rng().fill_bytes(&mut buf);

            if memory.write(&mut caller, out_ptr as usize, &buf).is_ok() {
                0
            } else {
                -1
            }
        },
    )?;

    // 3. Хеширование (SHA-256 / SHA-512)
    // alg: 0 = SHA-256 (32 байта output), 1 = SHA-512 (64 байта output)
    linker.func_wrap(
        "env",
        "host_hash",
        |mut caller: Caller<'_, HostState>,
         alg: i32,
         data_ptr: u32,
         data_len: u32,
         out_ptr: u32,
         out_max_len: u32| -> i32 {
            let Some(Extern::Memory(memory)) = caller.get_export("memory") else { return -1; };

            let expected_len = match alg {
                0 => 32, // SHA-256
                1 => 64, // SHA-512
                _ => return -2, // Неизвестный алгоритм
            };

            if (out_max_len as usize) < expected_len {
                return -3; // Буфер WASM слишком мал
            }

            // 1. Безопасно читаем вход без мутабельных заимствований
            let data = memory.data(&caller);
            let in_start = data_ptr as usize;
            let in_end = in_start.saturating_add(data_len as usize);

            if in_end > data.len() {
                return -1; // Выход за границы WASM памяти при чтении
            }

            let input = &data[in_start..in_end];

            // 2. Считаем хеш (без аллокаций в куче)
            let hash = match alg {
                0 => digest(&SHA256, input),
                1 => digest(&SHA512, input),
                _ => unreachable!(),
            };

            // 3. Записываем результат через memory.write (он сам проверит границы out_ptr)
            if memory.write(&mut caller, out_ptr as usize, hash.as_ref()).is_ok() {
                expected_len as i32
            } else {
                -1 // Ошибка записи (например, out_ptr за пределами памяти WASM)
            }
        },
    )?;

    Ok(())
}