use std::time::{SystemTime, UNIX_EPOCH};
use wasmtime::*;

pub fn register_time_api(linker: &mut Linker<()>) -> Result<()> {
    linker.func_wrap("env", "host_now_unix", |_caller: Caller<'_, ()>| -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    })?;

    linker.func_wrap("env", "host_now_millis", |_caller: Caller<'_, ()>| -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    })?;

    Ok(())
}