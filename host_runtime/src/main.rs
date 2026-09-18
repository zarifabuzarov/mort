mod files;
mod console;
mod process;
mod crypto;
mod time;
mod env_vars;
mod network;

use wasmtime::*;

fn main() -> Result<()> {
    println!("[Runtime] Запуск...");

    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    files::register_files_api(&mut linker)?;
    console::register_console_api(&mut linker)?;
    process::register_process_api(&mut linker)?;
    crypto::register_crypto_api(&mut linker)?;
    time::register_time_api(&mut linker)?;
    env_vars::register_env_api(&mut linker)?;
    network::register_network_api(&mut linker)?;

    let module = Module::from_file(&engine, "app.wasm")?;
    let instance = linker.instantiate(&mut store, &module)?;
    let start_func = instance.get_typed_func::<(), ()>(&mut store, "start")?;

    println!("[Runtime] Вызов функции из WASM:");
    start_func.call(&mut store, ())?;

    Ok(())
}