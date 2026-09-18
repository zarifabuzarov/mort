#![allow(dead_code)]
use std::str;


// Низкоуровневые импорты сидят внутри SDK и скрыты от разработчика
#[link(wasm_import_module = "env")]
extern "C" {
    fn host_print(ptr: *const u8, len: usize);
    fn host_sleep(mls: u64);
    fn host_random_u32() -> u32;
    fn host_now_unix() -> u64;
    fn host_now_millis() -> u64;
    fn host_get_env(key_ptr: *const u8, key_len: usize, out_ptr: *mut u8) -> u32;
    fn host_get_pid() -> u32;
    fn host_exit(code: i32) -> !;
    fn host_http_get(url_ptr: *const u8, url_len: usize, out_ptr: *mut u8, out_max_len: usize) -> u32;
    fn host_read_line(out_ptr: *mut u8, max_len: usize) -> u32;

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
}

// --- Безопасные абстракции ---

#[inline(always)]
pub fn print(s: &str) {
    unsafe { host_print(s.as_ptr(), s.len()) };
}

#[inline(always)]
pub fn println(s: &str) {
    print(s);
    print("\n");
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
    let mut buffer = [0u8; 4096];
    let len = unsafe { host_get_env(key.as_ptr(), key.len(), buffer.as_mut_ptr()) };
    if len > 0 {
        Some(String::from_utf8_lossy(&buffer[..len as usize]).to_string())
    } else {
        None
    }
}

#[inline(always)]
pub fn get_pid() -> u32 {
    unsafe { host_get_pid() }
}

#[inline(always)]
pub fn http_get(url: &str) -> Result<String, ()> {
    let mut buffer = [0u8; 4096];
    let len = unsafe { host_http_get(url.as_ptr(), url.len(), buffer.as_mut_ptr(), buffer.len()) };
    if len > 0 {
        Ok(String::from_utf8_lossy(&buffer[..len as usize]).to_string())
    } else {
        Err(())
    }
}

#[inline(always)]
pub fn exit(code: i32) -> ! {
    unsafe { host_exit(code) }
}