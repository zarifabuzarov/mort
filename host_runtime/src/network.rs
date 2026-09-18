use wasmtime::*;

pub fn register_network_api(linker: &mut Linker<()>) -> Result<()> {
    // host_http_get(url_ptr, url_len, out_ptr, out_max_len) -> u32
    linker.func_wrap(
        "env",
        "host_http_get",
        |mut caller: Caller<'_, ()>, url_ptr: u32, url_len: u32, out_ptr: u32, out_max_len: u32| -> u32 {
            let memory = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => return 0,
            };

            let data = memory.data(&caller);
            let url = match std::str::from_utf8(&data[url_ptr as usize..(url_ptr + url_len) as usize]) {
                Ok(s) => s,
                Err(_) => return 0,
            };

            // Делаем простой синхронный HTTP GET
            if let Ok(response) = ureq::get(url).call() {
                if let Ok(reader) = response.into_string() {
                    let bytes = reader.as_bytes();
                    let write_len = std::cmp::min(bytes.len(), out_max_len as usize);

                    let memory_mut = memory.data_mut(&mut caller);
                    let start = out_ptr as usize;
                    let end = start + write_len;

                    memory_mut[start..end].copy_from_slice(&bytes[..write_len]);
                    return write_len as u32;
                }
            }

            0
        },
    )?;

    Ok(())
}