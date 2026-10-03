use std::fs::{self, OpenOptions};
use std::io::{Read, Write, Seek};
use std::path::Path;
use wasmtime::*;

use crate::state::HostState;
use crate::utils::{
    read_bytes_from_mem, read_str_from_mem, with_mem_slice_mut, write_bytes_to_mem,
};

use std::os::unix::fs::PermissionsExt;
use std::time::UNIX_EPOCH;

pub fn register_files_api(linker: &mut Linker<HostState>) -> Result<()> {
    // host_open_file(path_ptr, path_len, mode) -> fd (или -1)
    // mode: 0 = read, 1 = write/truncate, 2 = append
    linker.func_wrap(
        "env",
        "host_open_file",
        |mut caller: Caller<'_, HostState>, path_ptr: i32, path_len: i32, mode: i32| -> i32 {
            let path = match read_str_from_mem(&mut caller, path_ptr, path_len) {
                Ok(p) => p,
                Err(_) => return -1,
            };

            let mut opts = OpenOptions::new();
            match mode {
                0 => { opts.read(true); }
                1 => { opts.write(true).create(true).truncate(true); }
                2 => { opts.write(true).create(true).append(true); }
                _ => return -1,
            };

            let file = match opts.open(&path) {
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
    // Используем Zero-Copy прямое чтение из файла в память WASM через with_mem_slice_mut!
    linker.func_wrap(
        "env",
        "host_read_file",
        |mut caller: Caller<'_, HostState>, fd: i32, buf_ptr: i32, buf_len: i32| -> i32 {
            // Сначала проверяем, есть ли такой открытый файл
            let mut file = match caller.data_mut().files.remove(&fd) {
                Some(f) => f,
                None => return -1,
            };

            // Читаем данные напрямую в срез памяти WASM (Zero-Copy без промежуточного vec![0; N]!)
            let read_result = with_mem_slice_mut(&mut caller, buf_ptr, buf_len, |buf| {
                file.read(buf)
            });

            // Возвращаем файл обратно в стейт
            let bytes_read = match read_result {
                Ok(Ok(n)) => n as i32,
                _ => -1,
            };

            caller.data_mut().files.insert(fd, file);
            bytes_read
        },
    )?;

    // host_write_file(fd, data_ptr, data_len) -> записано байт (или -1)
    linker.func_wrap(
        "env",
        "host_write_file",
        |mut caller: Caller<'_, HostState>, fd: i32, data_ptr: i32, data_len: i32| -> i32 {
            let bytes_to_write = match read_bytes_from_mem(&mut caller, data_ptr, data_len) {
                Ok(b) => b,
                Err(_) => return -1,
            };

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
    linker.func_wrap(
        "env",
        "host_path_exists",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
            let Ok(path) = read_str_from_mem(&mut caller, ptr, len) else {
                return -1;
            };
            if Path::new(&path).exists() { 1 } else { 0 }
        },
    )?;

    linker.func_wrap(
        "env",
        "host_path_is_file",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
            let Ok(path) = read_str_from_mem(&mut caller, ptr, len) else {
                return -1;
            };
            if Path::new(&path).is_file() { 1 } else { 0 }
        },
    )?;

    linker.func_wrap(
        "env",
        "host_path_is_dir",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
            let Ok(path) = read_str_from_mem(&mut caller, ptr, len) else {
                return -1;
            };
            if Path::new(&path).is_dir() { 1 } else { 0 }
        },
    )?;

    // 2. Создание файлов и директорий
    linker.func_wrap(
        "env",
        "host_make_file",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
            let Ok(path) = read_str_from_mem(&mut caller, ptr, len) else {
                return -1;
            };
            match fs::File::create(path) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "host_make_dir",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32, recursive: i32| -> i32 {
            let Ok(path) = read_str_from_mem(&mut caller, ptr, len) else {
                return -1;
            };
            let res = if recursive != 0 {
                fs::create_dir_all(path)
            } else {
                fs::create_dir(path)
            };
            if res.is_ok() { 0 } else { -1 }
        },
    )?;

    // 3. Удаление
    linker.func_wrap(
        "env",
        "host_remove_file",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
            let Ok(path) = read_str_from_mem(&mut caller, ptr, len) else {
                return -1;
            };
            if fs::remove_file(path).is_ok() { 0 } else { -1 }
        },
    )?;

    linker.func_wrap(
        "env",
        "host_remove_dir",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32, recursive: i32| -> i32 {
            let Ok(path) = read_str_from_mem(&mut caller, ptr, len) else {
                return -1;
            };
            let res = if recursive != 0 {
                fs::remove_dir_all(path)
            } else {
                fs::remove_dir(path)
            };
            if res.is_ok() { 0 } else { -1 }
        },
    )?;

    // 4. Чтение списка файлов (выводит разделенные \n имена)
    linker.func_wrap(
        "env",
        "host_list_dir",
        |mut caller: Caller<'_, HostState>,
         path_ptr: i32,
         path_len: i32,
         out_ptr: i32,
         max_len: i32|
         -> i32 {
            let Ok(path) = read_str_from_mem(&mut caller, path_ptr, path_len) else {
                return -1;
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

            let bytes = result.as_bytes();

            // ПРОВЕРКА: Если список файлов больше, чем буфер WASM — отдаем ошибку
            if bytes.len() > max_len as usize {
                return -1;
            }

            // Записываем только если точно влезает
            match write_bytes_to_mem(&mut caller, out_ptr, bytes) {
                Ok(_) => bytes.len() as i32,
                Err(_) => -1,
            }
        },
    )?;

    // 1. Узнать размер файла
    linker.func_wrap(
        "env",
        "host_file_size",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i64 {
            let Ok(path) = read_str_from_mem(&mut caller, ptr, len) else {
                return -1;
            };
            fs::metadata(path).map(|m| m.len() as i64).unwrap_or(-1)
        },
    )?;

    // 2. Переименовать / Переместить
    linker.func_wrap(
        "env",
        "host_rename",
        |mut caller: Caller<'_, HostState>,
         from_ptr: i32, from_len: i32,
         to_ptr: i32, to_len: i32| -> i32 {
            let Ok(from) = read_str_from_mem(&mut caller, from_ptr, from_len) else { return -1; };
            let Ok(to) = read_str_from_mem(&mut caller, to_ptr, to_len) else { return -1; };

            // Пробуем быстрое атомарное переименование
            if fs::rename(&from, &to).is_ok() {
                return 0;
            }

            // Если разные диски/FS — копируем и удаляем оригинал
            if fs::copy(&from, &to).is_ok() {
                if fs::remove_file(&from).is_ok() {
                    return 0;
                }
            }

            -1
        },
    )?;

    // 3. Смещение курсора чтения/записи (Seek)
    linker.func_wrap(
        "env",
        "host_seek_file",
        |mut caller: Caller<'_, HostState>, fd: i32, offset: i64| -> i64 {
            let state = caller.data_mut();
            let Some(file) = state.files.get_mut(&fd) else { return -1; };

            file.seek(std::io::SeekFrom::Start(offset as u64))
                .map(|new_pos| new_pos as i64)
                .unwrap_or(-1)
        },
    )?;

    linker.func_wrap(
        "env",
        "host_current_dir",
        |mut caller: Caller<'_, HostState>, out_ptr: i32, max_len: i32| -> i32 {
            let Ok(cwd) = std::env::current_dir() else { return -1; };
            let cwd_str = cwd.to_string_lossy();
            let bytes = cwd_str.as_bytes();

            if bytes.len() > max_len as usize {
                return -1;
            }

            match write_bytes_to_mem(&mut caller, out_ptr, bytes) {
                Ok(_) => bytes.len() as i32,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "host_set_current_dir",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
            let Ok(path) = read_str_from_mem(&mut caller, ptr, len) else { return -1; };
            if std::env::set_current_dir(path).is_ok() { 0 } else { -1 }
        },
    )?;

    linker.func_wrap(
        "env",
        "host_stat",
        |mut caller: Caller<'_, HostState>, path_ptr: i32, path_len: i32, out_stat_ptr: i32| -> i32 {
            let Ok(path) = read_str_from_mem(&mut caller, path_ptr, path_len) else { return -1; };
            let Ok(meta) = fs::metadata(&path) else { return -1; };

            let created = meta.created().ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs()).unwrap_or(0);

            let modified = meta.modified().ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs()).unwrap_or(0);

            let accessed = meta.accessed().ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs()).unwrap_or(0);

            #[cfg(unix)]
            let mode = meta.permissions().mode();
            #[cfg(not(unix))]
            let mode = if meta.permissions().readonly() { 0o444 } else { 0o666 };

            let is_readonly = if meta.permissions().readonly() { 1u32 } else { 0u32 };

            // Собираем буфер из 48 байт (u64 x 4 + u32 x 2)
            let mut bytes = Vec::with_capacity(48);
            bytes.extend_from_slice(&meta.len().to_le_bytes());
            bytes.extend_from_slice(&created.to_le_bytes());
            bytes.extend_from_slice(&modified.to_le_bytes());
            bytes.extend_from_slice(&accessed.to_le_bytes());
            bytes.extend_from_slice(&mode.to_le_bytes());
            bytes.extend_from_slice(&is_readonly.to_le_bytes());

            match write_bytes_to_mem(&mut caller, out_stat_ptr, &bytes) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        },
    )?;

    Ok(())
}