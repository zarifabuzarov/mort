use wasmtime::*;
use std::process;
use crate::state::HostState;

pub fn register_process_api(linker: &mut Linker<HostState>) -> Result<()> {
    // Получение PID текущего хост-процесса
    linker.func_wrap("env", "host_get_pid", |_caller: Caller<'_, HostState>| -> u32 {
        process::id()
    })?;

    // Завершение работы WASM-модуля
    linker.func_wrap(
        "env",
        "host_exit",
        |_caller: Caller<'_, HostState>, code: i32| -> Result<(), Error> {
            println!("[Host Process] Выход из WASM с кодом: {}", code);

            if code == 0 {
                // Мягко прерываем исполнение WASM-модуля (не роняя сам хост!)
                Err(Error::msg("WASM_EXIT_SUCCESS"))
            } else {
                Err(Error::msg(format!("WASM_EXIT_FAILURE: {code}")))
            }
        }
    )?;

    Ok(())
}