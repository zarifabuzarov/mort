#![allow(dead_code)]
use std::str;

// Низкоуровневые импорты сидят внутри SDK и скрыты от разработчика
#[link(wasm_import_module = "env")]
extern "C" {
    fn host_print(ptr: *const u8, len: usize);
    fn host_print_err(ptr: *const u8, len: usize);
    fn host_sleep(mls: u64);
    fn host_random_u32() -> u32;
    fn host_now_unix() -> u64;
    fn host_now_millis() -> u64;
    fn host_get_env_len(key_ptr: *const u8, key_len: usize) -> u32;
    fn host_get_env(key_ptr: *const u8, key_len: usize, out_ptr: *mut u8, out_max_len: usize) -> u32;
    fn host_get_pid() -> u32;
    fn host_exit(code: i32) -> !;

    // Обновленный HTTP API
    fn host_http_request(
        method_ptr: *const u8, method_len: usize,
        url_ptr: *const u8, url_len: usize,
        body_ptr: *const u8, body_len: usize,
        out_ptr: *mut u8, max_len: usize,
        status_out_ptr: *mut i32,
    ) -> i32;

    fn host_read_line(out_ptr: *mut u8, max_len: usize) -> i32;

    // Файловый API
    fn host_open_file(path_ptr: *const u8, path_len: usize, mode: i32) -> i32;
    fn host_read_file(fd: i32, buf_ptr: *mut u8, buf_len: usize) -> i32;
    fn host_write_file(fd: i32, data_ptr: *const u8, data_len: usize) -> i32;
    fn host_close_file(fd: i32) -> i32;

    fn host_path_exists(ptr: *const u8, len: usize) -> i32;
    fn host_path_is_file(ptr: *const u8, len: usize) -> i32;
    fn host_path_is_dir(ptr: *const u8, len: usize) -> i32;
    fn host_make_file(ptr: *const u8, len: usize) -> i32;
    fn host_make_dir(ptr: *const u8, len: usize, recursive: i32) -> i32;
    fn host_remove_file(ptr: *const u8, len: usize) -> i32;
    fn host_remove_dir(ptr: *const u8, len: usize, recursive: i32) -> i32;
    fn host_list_dir(path_ptr: *const u8, path_len: usize, out_ptr: *mut u8, max_len: usize) -> i32;

    // Новые функции ФС
    fn host_file_size(ptr: *const u8, len: usize) -> i64;
    fn host_rename(from_ptr: *const u8, from_len: usize, to_ptr: *const u8, to_len: usize) -> i32;
    fn host_seek_file(fd: i32, offset: i64) -> i64;

    fn host_random_bytes(out_ptr: *mut u8, len: usize) -> i32;
    fn host_hash(
        alg: i32,
        data_ptr: *const u8,
        data_len: usize,
        out_ptr: *mut u8,
        out_max_len: usize,
    ) -> i32;

    fn host_image_info(img_ptr: *const u8, img_len: usize, out_w: *mut u32, out_h: *mut u32) -> i32;
    fn host_decode_image(img_ptr: *const u8, img_len: usize, out_rgba_ptr: *mut u8, max_len: usize) -> i32;

    fn host_process_image(
            src_ptr: *const u8,
            src_len: usize,
            transform_ptr: *const ImageTransform,
            out_ptr: *mut u8,
            out_max_len: usize,
        ) -> i32;
}

// --- Структуры данных ---

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: i32,
    pub body: String,
}

// --- Безопасные абстракции ---

#[inline(always)]
pub fn print(s: &str) {
    unsafe { host_print(s.as_ptr(), s.len()) };
}

#[inline(always)]
pub fn print_err(s: &str) {
    unsafe { host_print_err(s.as_ptr(), s.len()) };
}

#[inline(always)]
pub fn println(s: &str) {
    print(s);
    print("\n");
}

#[inline(always)]
pub fn eprintln(s: &str) {
    print_err(s);
    print_err("\n");
}

#[inline(always)]
pub fn read_line() -> String {
    let mut buf = [0u8; 256];
    let len = unsafe { host_read_line(buf.as_mut_ptr(), buf.len()) };
    if len > 0 {
        String::from_utf8_lossy(&buf[..len as usize]).to_string()
    } else {
        String::new()
    }
}

pub struct FileHandle {
    fd: i32,
}

impl FileHandle {
    pub fn open_read(path: &str) -> Result<Self, ()> {
        let fd = unsafe { host_open_file(path.as_ptr(), path.len(), 0) };
        if fd >= 0 { Ok(Self { fd }) } else { Err(()) }
    }

    pub fn create_write(path: &str) -> Result<Self, ()> {
        let fd = unsafe { host_open_file(path.as_ptr(), path.len(), 1) };
        if fd >= 0 { Ok(Self { fd }) } else { Err(()) }
    }

    pub fn open_append(path: &str) -> Result<Self, ()> {
        let fd = unsafe { host_open_file(path.as_ptr(), path.len(), 2) };
        if fd >= 0 { Ok(Self { fd }) } else { Err(()) }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        let res = unsafe { host_read_file(self.fd, buf.as_mut_ptr(), buf.len()) };
        if res >= 0 { Ok(res as usize) } else { Err(()) }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize, ()> {
        let res = unsafe { host_write_file(self.fd, data.as_ptr(), data.len()) };
        if res >= 0 { Ok(res as usize) } else { Err(()) }
    }

    /// Смещение курсора файла на абсолютную позицию (Seek from start)
    pub fn seek(&mut self, offset: u64) -> Result<u64, ()> {
        let res = unsafe { host_seek_file(self.fd, offset as i64) };
        if res >= 0 { Ok(res as u64) } else { Err(()) }
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        unsafe { host_close_file(self.fd); }
    }
}

pub fn path_exists(path: &str) -> bool {
    unsafe { host_path_exists(path.as_ptr(), path.len()) == 1 }
}

pub fn is_file(path: &str) -> bool {
    unsafe { host_path_is_file(path.as_ptr(), path.len()) == 1 }
}

pub fn is_dir(path: &str) -> bool {
    unsafe { host_path_is_dir(path.as_ptr(), path.len()) == 1 }
}

pub fn make_file(path: &str) -> bool {
    unsafe { host_make_file(path.as_ptr(), path.len()) == 0 }
}

pub fn make_dir(path: &str, recursive: bool) -> bool {
    unsafe { host_make_dir(path.as_ptr(), path.len(), if recursive { 1 } else { 0 }) == 0 }
}

pub fn remove_file(path: &str) -> bool {
    unsafe { host_remove_file(path.as_ptr(), path.len()) == 0 }
}

pub fn remove_dir(path: &str, recursive: bool) -> bool {
    unsafe { host_remove_dir(path.as_ptr(), path.len(), if recursive { 1 } else { 0 }) == 0 }
}

pub fn list_dir(path: &str) -> Result<Vec<String>, ()> {
    let mut buf = [0u8; 4096];
    let len = unsafe { host_list_dir(path.as_ptr(), path.len(), buf.as_mut_ptr(), buf.len()) };
    if len < 0 {
        return Err(());
    }
    let s = String::from_utf8_lossy(&buf[..len as usize]);
    Ok(s.lines().map(|line| line.to_string()).collect())
}

/// Получение размера файла в байтах
pub fn file_size(path: &str) -> Option<u64> {
    let res = unsafe { host_file_size(path.as_ptr(), path.len()) };
    if res >= 0 {
        Some(res as u64)
    } else {
        None
    }
}

/// Переименование / перемещение файла или директории
pub fn rename(from: &str, to: &str) -> bool {
    unsafe { host_rename(from.as_ptr(), from.len(), to.as_ptr(), to.len()) == 0 }
}

#[inline(always)]
pub fn sleep(ms: u64) {
    unsafe { host_sleep(ms) };
}

#[inline(always)]
pub fn random_u32() -> u32 {
    unsafe { host_random_u32() }
}

#[inline(always)]
pub fn now_unix() -> u64 {
    unsafe { host_now_unix() }
}

#[inline(always)]
pub fn now_millis() -> u64 {
    unsafe { host_now_millis() }
}

#[inline(always)]
pub fn get_env(key: &str) -> Option<String> {
//     print("[SDK Debug] Заходим в get_env...\n");//
    let key_bytes = key.as_bytes();

    let len = unsafe {
        host_get_env_len(key_bytes.as_ptr(), key_bytes.len())
    };

//     println(&format!("[SDK Debug] Длина от хоста: {}"/, len));

    if len == 0 {
        return None;
    }

    let mut buf = [0u8; 4096];
    let max_len = buf.len().min(len as usize);

//     print("[SDK Debug] Вызываем host_get_env...\n");
    let read = unsafe {
        host_get_env(key_bytes.as_ptr(), key_bytes.len(), buf.as_mut_ptr(), max_len)
    };

//     println(&format!("[SDK Debug] Прочитано байт: {}", read));

    if read > 0 {
        String::from_utf8(buf[..read as usize].to_vec()).ok()
    } else {
        None
    }
}

#[inline(always)]
pub fn get_pid() -> u32 {
    unsafe { host_get_pid() }
}

// Универсальный HTTP запрос
pub fn http_request(method: &str, url: &str, body: Option<&str>) -> Result<HttpResponse, ()> {
    let mut response_buf = [0u8; 8192];
    let mut status_code: i32 = 0;

    let body_bytes = body.unwrap_or("").as_bytes();

    let bytes_read = unsafe {
        host_http_request(
            method.as_ptr(),
            method.len(),
            url.as_ptr(),
            url.len(),
            body_bytes.as_ptr(),
            body_bytes.len(),
            response_buf.as_mut_ptr(),
            response_buf.len(),
            &mut status_code as *mut i32,
        )
    };

    if bytes_read < 0 {
        return Err(());
    }

    let body_str = String::from_utf8_lossy(&response_buf[..bytes_read as usize]).to_string();

    Ok(HttpResponse {
        status: status_code,
        body: body_str,
    })
}

// Обертка для GET
#[inline(always)]
pub fn http_get(url: &str) -> Result<HttpResponse, ()> {
    http_request("GET", url, None)
}

// Обертка для POST
#[inline(always)]
pub fn http_post(url: &str, body: &str) -> Result<HttpResponse, ()> {
    http_request("POST", url, Some(body))
}

#[inline(always)]
pub fn exit(code: i32) -> ! {
    unsafe { host_exit(code) }
}

pub fn random_bytes(buf: &mut [u8]) -> bool {
    unsafe { host_random_bytes(buf.as_mut_ptr(), buf.len()) == 0 }
}

pub fn sha256(data: &[u8]) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    let res = unsafe {
        host_hash(0, data.as_ptr(), data.len(), out.as_mut_ptr(), out.len())
    };
    if res == 32 { Some(out) } else { None }
}

pub fn sha512(data: &[u8]) -> Option<[u8; 64]> {
    let mut out = [0u8; 64];
    let res = unsafe {
        host_hash(1, data.as_ptr(), data.len(), out.as_mut_ptr(), out.len())
    };
    if res == 64 { Some(out) } else { None }
}

pub struct ImageFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA байты
}

pub fn decode_image(encoded_bytes: &[u8]) -> Option<ImageFrame> {
    let mut width: u32 = 0;
    let mut height: u32 = 0;

    // 1. Получаем размеры
    let res = unsafe {
        host_image_info(encoded_bytes.as_ptr(), encoded_bytes.len(), &mut width, &mut height)
    };
    if res != 0 { return None; }

    // 2. Выделяем буфер под RGBA (Width * Height * 4 байта)
    let required_size = (width * height * 4) as usize;
    let mut pixels = vec![0u8; required_size];

    // 3. Декодируем
    let decoded_size = unsafe {
        host_decode_image(
            encoded_bytes.as_ptr(),
            encoded_bytes.len(),
            pixels.as_mut_ptr(),
            pixels.len(),
        )
    };

    if decoded_size > 0 {
        Some(ImageFrame { width, height, pixels })
    } else {
        None
    }
}

#[repr(C)]
pub struct ImageTransform {
    pub resize_width: u32,   // 0 = не менять
    pub resize_height: u32,  // 0 = не менять
    pub rotate_degrees: u16, // 0, 90, 180, 270
    pub crop_x: u32,
    pub crop_y: u32,
    pub crop_w: u32,         // 0 = без кропа
    pub crop_h: u32,

    pub fn new_resize(w: u32, h: u32) -> Self {
        Self {
            resize_width: w,
            resize_height: h,
            rotate_degrees: 0,
            crop_x: 0, crop_y: 0, crop_w: 0, crop_h: 0,
        }
    }

    pub fn with_rotate(mut self, deg: u16) -> Self {
        self.rotate_degrees = deg;
        self
    }
}

/// Удобная обертка для Гостя
pub fn process_image(input_bytes: &[u8], transform: &ImageTransform) -> Option<Vec<u8>> {
    // Выделяем буфер под выходной файл с запасом (например, равным размеру оригинала)
    let mut out_buf = vec![0u8; input_bytes.len().max(1024 * 1024)];

    let written = unsafe {
        host_process_image(
            input_bytes.as_ptr(),
            input_bytes.len(),
            transform,
            out_buf.as_mut_ptr(),
            out_buf.len(),
        )
    };

    if written > 0 {
        out_buf.truncate(written as usize);
        Some(out_buf)
    } else {
        None
    }
}

// В твоем sdk/fs.rs
pub fn read<P: AsRef<str>>(path: P) -> Result<Vec<u8>, FsError> {
    let mut file = FileHandle::open_read(path.as_ref())?;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192]; // 8КБ читаются быстрее
    while let Ok(n) = file.read(&mut chunk) {
        if n == 0 { break; }
        bytes.extend_from_slice(&chunk[..n]);
    }
    Ok(bytes)
}

pub fn write<P: AsRef<str>>(path: P, contents: &[u8]) -> Result<(), FsError> {
    let mut file = FileHandle::create_write(path.as_ref())?;
    file.write(contents)?;
    Ok(())
}