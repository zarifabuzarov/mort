use std::str;

// Низкоуровневые импорты сидят внутри SDK и скрыты от разработчика
#[link(wasm_import_module = "env")]
extern "C" {
    fn host_print(ptr: *const u8, len: usize);
    fn host_read_file(path_ptr: *const u8, path_len: usize, buf_ptr: *mut u8, buf_max_len: usize) -> i32;
    fn host_write_file(path_ptr: *const u8, path_len: usize, data_ptr: *const u8, data_len: usize) -> i32;
    fn host_append_file(path_ptr: *const u8, path_len: usize, data_ptr: *const u8, data_len: usize) -> i32;
    fn host_sleep(mls: u64);
    fn host_random_u32() -> u32;
    fn host_now_unix() -> u64;
    fn host_now_millis() -> u64;
    fn host_get_env(key_ptr: *const u8, key_len: usize, out_ptr: *mut u8) -> u32;
    fn host_get_pid() -> u32;
    fn host_exit(code: i32) -> !;
    fn host_http_get(url_ptr: *const u8, url_len: usize, out_ptr: *mut u8, out_max_len: usize) -> u32;
    fn host_read_line(out_ptr: *mut u8, max_len: usize) -> u32;
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

#[inline(always)]
pub fn write_file(path: &str, data: &str) -> bool {
    unsafe { host_write_file(path.as_ptr(), path.len(), data.as_ptr(), data.len()) == 0 }
}

#[inline(always)]
pub fn append_file(path: &str, data: &str) -> bool {
    unsafe { host_append_file(path.as_ptr(), path.len(), data.as_ptr(), data.len()) == 0 }
}

#[inline(always)]
pub fn read_file(path: &str) -> Result<String, ()> {
    let mut buffer = [0u8; 4096];
    let bytes_read = unsafe {
        host_read_file(path.as_ptr(), path.len(), buffer.as_mut_ptr(), buffer.len())
    };
    if bytes_read >= 0 {
        Ok(String::from_utf8_lossy(&buffer[..bytes_read as usize]).to_string())
    } else {
        Err(())
    }
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