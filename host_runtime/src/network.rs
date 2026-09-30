use wasmtime::*;
use crate::state::HostState;
use crate::utils::{read_str_from_mem, write_bytes_to_mem, with_mem_slice_mut};
use std::io::Read;

pub fn register_network_api(linker: &mut Linker<HostState>) -> Result<()> {
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

            let body_bytes = if body_len > 0 {
                let Ok(body_str) = read_str_from_mem(&mut caller, body_ptr, body_len) else { return -1; };
                body_str.into_bytes()
            } else {
                Vec::new()
            };

            let req = ureq::request(&method.to_uppercase(), &url)
                .timeout(std::time::Duration::from_secs(10));

            let response_result = if body_bytes.is_empty() {
                req.call()
            } else {
                req.send_bytes(&body_bytes)
            };

            let (status, mut reader) = match response_result {
                Ok(resp) => (resp.status() as i32, resp.into_reader()),
                Err(ureq::Error::Status(code, resp)) => (code as i32, resp.into_reader()),
                Err(_) => return -1,
            };

            let status_bytes = status.to_le_bytes();
            if write_bytes_to_mem(&mut caller, status_out_ptr, &status_bytes).is_err() {
                return -1;
            }

            // Читаем из сокета прямо в WASM-память без лишнего промежуточного vec![0u8; N]
            let read_result = with_mem_slice_mut(&mut caller, out_ptr, max_len, |buf| {
                reader.read(buf)
            });

            match read_result {
                Ok(Ok(n)) => n as i32,
                _ => -1,
            }
        },
    )?;

    Ok(())
}