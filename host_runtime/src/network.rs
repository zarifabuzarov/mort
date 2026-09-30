use wasmtime::*;
use crate::state::HostState;
use crate::utils::{read_str_from_mem, write_bytes_to_mem};
use std::io::Read;

pub fn register_network_api(linker: &mut Linker<HostState>) -> Result<()> {

    // Универсальный HTTP запрос
    // method: "GET", "POST", "PUT", "DELETE"
    // status_out_ptr: указатель на i32 в WASM, куда запишется HTTP-код (200, 404, 500)
    // Возвращает: количество записанных байт тела ответа или -1 при ошибке
    linker.func_wrap(
        "env",
        "host_http_request",
        |mut caller: Caller<'_, HostState>,
         method_ptr: i32, method_len: i32,
         url_ptr: i32, url_len: i32,
         body_ptr: i32, body_len: i32,
         out_ptr: i32, max_len: i32,
         status_out_ptr: i32| -> i32 {

            let Ok(method) = read_str_from_mem(&mut caller, method_ptr, method_len) else { return -1; };
            let Ok(url) = read_str_from_mem(&mut caller, url_ptr, url_len) else { return -1; };

            // Читаем тело запроса (если есть)
            let body_bytes = if body_len > 0 {
                let Ok(body_str) = read_str_from_mem(&mut caller, body_ptr, body_len) else { return -1; };
                body_str.into_bytes()
            } else {
                Vec::new()
            };

            // Собираем HTTP запрос с таймаутом 10 секунд
            let req = ureq::request(&method.to_uppercase(), &url)
                .timeout(std::time::Duration::from_secs(10));

            let response_result = if body_bytes.is_empty() {
                req.call()
            } else {
                req.send_bytes(&body_bytes)
            };

            // Разбираем ответ или сетевую ошибку
            let (status, mut reader) = match response_result {
                Ok(resp) => (resp.status() as i32, resp.into_reader()),
                Err(ureq::Error::Status(code, resp)) => (code as i32, resp.into_reader()),
                Err(_) => return -1, // Ошибка сети / DNS / Таймаут
            };

            // Записываем HTTP статус обратно в память WASM (4 байта i32)
            let status_bytes = status.to_le_bytes();
            if write_bytes_to_mem(&mut caller, status_out_ptr, &status_bytes).is_err() {
                return -1;
            }

            // Вычитываем тело ответа напрямую в буфер WASM
            let mut response_buf = vec![0u8; max_len as usize];
            let read_bytes = match reader.read(&mut response_buf) {
                Ok(n) => n,
                Err(_) => return -1,
            };

            if write_bytes_to_mem(&mut caller, out_ptr, &response_buf[..read_bytes]).is_err() {
                return -1;
            }

            read_bytes as i32
        },
    )?;

    Ok(())
}