use std::fs::{self, OpenOptions};
use std::io::Write;
use wasmtime::*;

pub fn register_files_api(linker: &mut Linker<()>) -> Result<()> {
    // 1. Чтение файла
    linker.func_wrap(
        "env",
        "host_read_file",
        |mut caller: Caller<'_, ()>, path_ptr: i32, path_len: i32, buf_ptr: i32, buf_max_len: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => return -1,
            };

            let data = memory.data(&caller);
            let path = match std::str::from_utf8(&data[path_ptr as usize..(path_ptr + path_len) as usize]) {
                Ok(p) => p,
                Err(_) => return -1,
            };

            let content = match fs::read(path) {
                Ok(bytes) => bytes,
                Err(_) => return -1,
            };

            if content.len() > buf_max_len as usize {
                return -1;
            }

            let b_start = buf_ptr as usize;
            let memory_mut = memory.data_mut(&mut caller);
            memory_mut[b_start..b_start + content.len()].copy_from_slice(&content);

            content.len() as i32
        },
    )?;

    // 2. Перезапись файла с нуля (Overwrite)
    linker.func_wrap(
        "env",
        "host_write_file",
        |mut caller: Caller<'_, ()>, path_ptr: i32, path_len: i32, data_ptr: i32, data_len: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => return -1,
            };

            let data = memory.data(&caller);
            let path = match std::str::from_utf8(&data[path_ptr as usize..(path_ptr + path_len) as usize]) {
                Ok(p) => p,
                Err(_) => return -1,
            };

            let bytes_to_write = &data[data_ptr as usize..(data_ptr + data_len) as usize];

            let mut file = match OpenOptions::new().write(true).create(true).truncate(true).open(path) {
                Ok(f) => f,
                Err(_) => return -1,
            };

            match file.write_all(bytes_to_write) {
                Ok(_) => bytes_to_write.len() as i32,
                Err(_) => -1,
            }
        },
    )?;

    // 3. Дозапись в конец файла (Append)
    linker.func_wrap(
        "env",
        "host_append_file",
        |mut caller: Caller<'_, ()>, path_ptr: i32, path_len: i32, data_ptr: i32, data_len: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => return -1,
            };

            let data = memory.data(&caller);
            let path = match std::str::from_utf8(&data[path_ptr as usize..(path_ptr + path_len) as usize]) {
                Ok(p) => p,
                Err(_) => return -1,
            };

            let bytes_to_write = &data[data_ptr as usize..(data_ptr + data_len) as usize];

            let mut file = match OpenOptions::new().write(true).create(true).append(true).open(path) {
                Ok(f) => f,
                Err(_) => return -1,
            };

            match file.write_all(bytes_to_write) {
                Ok(_) => bytes_to_write.len() as i32,
                Err(_) => -1,
            }
        },
    )?;

    Ok(())
}