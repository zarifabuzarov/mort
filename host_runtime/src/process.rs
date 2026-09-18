use wasmtime::*;
use std::thread::sleep;
use std::time::Duration;
use std::process;
use crate::state::HostState;

pub fn register_process_api(linker: &mut Linker<HostState>) -> Result<()> {
    
    linker.func_wrap("env", "host_sleep", |_caller: Caller<'_, HostState>, millis: u64| {
        sleep(Duration::from_millis(millis));
    })?;

    // Получение PID
    linker.func_wrap("env", "host_get_pid", |_caller: Caller<'_, HostState>| -> u32 {
        process::id()
    })?;

    // Завершение работы процесса
    linker.func_wrap("env", "host_exit", |_caller: Caller<'_, HostState>, code: i32| -> () {
        println!("[Host Process] Выход из WASM с кодом: {}", code);
        process::exit(code);
    })?;

    Ok(())
}