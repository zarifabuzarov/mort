use std::fs::OpenOptions;
use std::io::{Read, Write};
use wasmtime::*;
use std::fs;
use std::path::Path;
use crate::state::HostState;
use crate::utils::read_str_from_mem;

pub fn register_files_api(linker: &mut Linker<HostState>) -> Result<()> {
    // host_open_file(path_ptr, path_len, mode) -> fd (или -1)
    // mode: 0 = read, 1 = write/truncate, 2 = append
    linker.func_wrap(
        "env",
        "host_open_file",
        |mut caller: Caller<'_, HostState>, path_ptr: i32, path_len: i32, mode: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => return -1,
            };

            let data = memory.data(&caller);
            let path = match std::str::from_utf8(&data[path_ptr as usize..(path_ptr + path_len) as usize]) {
                Ok(p) => p,
                Err(_) => return -1,
            };

            let mut opts = OpenOptions::new();
            match mode {
                0 => { opts.read(true); },
                1 => { opts.write(true).create(true).truncate(true); },
                2 => { opts.write(true).create(true).append(true); },
                _ => return -1,
            };

            let file = match opts.open(path) {
                Ok(f) => f,
                Err(_) => return -1,
            };

            let state = caller.data_mut();
            let fd = state.next_fd;
            state.next_fd += 1;
            state.files.insert(fd, file);

            fd
        },
    )?;

    // host_read_file(fd, buf_ptr, buf_len) -> прочитано байт (или -1)
    linker.func_wrap(
        "env",
        "host_read_file",
        |mut caller: Caller<'_, HostState>, fd: i32, buf_ptr: i32, buf_len: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => return -1,
            };

            let state = caller.data_mut();
            let file = match state.files.get_mut(&fd) {
                Some(f) => f,
                None => return -1,
            };

            let mut chunk = vec![0u8; buf_len as usize];
            let bytes_read = match file.read(&mut chunk) {
                Ok(n) => n,
                Err(_) => return -1,
            };

            let memory_mut = memory.data_mut(&mut caller);
            let start = buf_ptr as usize;
            memory_mut[start..start + bytes_read].copy_from_slice(&chunk[..bytes_read]);

            bytes_read as i32
        },
    )?;

    // host_write_file(fd, data_ptr, data_len) -> записано байт (или -1)
    linker.func_wrap(
        "env",
        "host_write_file",
        |mut caller: Caller<'_, HostState>, fd: i32, data_ptr: i32, data_len: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => return -1,
            };

            let data = memory.data(&caller);
            let start = data_ptr as usize;
            let bytes_to_write = data[start..start + data_len as usize].to_vec();

            let state = caller.data_mut();
            let file = match state.files.get_mut(&fd) {
                Some(f) => f,
                None => return -1,
            };

            match file.write(&bytes_to_write) {
                Ok(n) => n as i32,
                Err(_) => -1,
            }
        },
    )?;

    // host_close_file(fd) -> 0 или -1
    linker.func_wrap(
        "env",
        "host_close_file",
        |mut caller: Caller<'_, HostState>, fd: i32| -> i32 {
            if caller.data_mut().files.remove(&fd).is_some() {
                0
            } else {
                -1
            }
        },
    )?;


    // 1. Проверки существования и типов
    linker.func_wrap("env", "host_path_exists", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
        let path = match read_str_from_mem(&mut caller, ptr, len) {
            Some(p) => p,
            None => return -1,
        };
        if Path::new(&path).exists() { 1 } else { 0 }
    })?;

    linker.func_wrap("env", "host_path_is_file", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
        let path = match read_str_from_mem(&mut caller, ptr, len) {
            Some(p) => p,
            None => return -1,
        };
        if Path::new(&path).is_file() { 1 } else { 0 }
    })?;

    linker.func_wrap("env", "host_path_is_dir", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
        let path = match read_str_from_mem(&mut caller, ptr, len) {
            Some(p) => p,
            None => return -1,
        };
        if Path::new(&path).is_dir() { 1 } else { 0 }
    })?;

    // 2. Создание файлов и директорий
    linker.func_wrap("env", "host_make_file", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
        let path = match read_str_from_mem(&mut caller, ptr, len) {
            Some(p) => p,
            None => return -1,
        };
        match fs::File::create(path) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    })?;

    linker.func_wrap("env", "host_make_dir", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32, recursive: i32| -> i32 {
        let path = match read_str_from_mem(&mut caller, ptr, len) {
            Some(p) => p,
            None => return -1,
        };
        let res = if recursive != 0 {
            fs::create_dir_all(path)
        } else {
            fs::create_dir(path)
        };
        if res.is_ok() { 0 } else { -1 }
    })?;

    // 3. Удаление
    linker.func_wrap("env", "host_remove_file", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
        let path = match read_str_from_mem(&mut caller, ptr, len) {
            Some(p) => p,
            None => return -1,
        };
        if fs::remove_file(path).is_ok() { 0 } else { -1 }
    })?;

    linker.func_wrap("env", "host_remove_dir", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32, recursive: i32| -> i32 {
        let path = match read_str_from_mem(&mut caller, ptr, len) {
            Some(p) => p,
            None => return -1,
        };
        let res = if recursive != 0 {
            fs::remove_dir_all(path)
        } else {
            fs::remove_dir(path)
        };
        if res.is_ok() { 0 } else { -1 }
    })?;

    // 4. Чтение списка файлов (выводит разделенные \n имена)
    linker.func_wrap("env", "host_list_dir", |mut caller: Caller<'_, HostState>, path_ptr: i32, path_len: i32, out_ptr: i32, max_len: i32| -> i32 {
        let path = match read_str_from_mem(&mut caller, path_ptr, path_len) {
            Some(p) => p,
            None => return -1,
        };

        let entries = match fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return -1,
        };

        let mut result = String::new();
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                result.push_str(name);
                result.push('\n');
            }
        }

        let memory = match caller.get_export("memory") {
            Some(Extern::Memory(mem)) => mem,
            _ => return -1,
        };

        let bytes = result.as_bytes();
        if bytes.len() > max_len as usize {
            return -1;
        }

        let memory_mut = memory.data_mut(&mut caller);
        let start = out_ptr as usize;
        memory_mut[start..start + bytes.len()].copy_from_slice(bytes);

        bytes.len() as i32
    })?;

    Ok(())
}