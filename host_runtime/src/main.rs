mod state;
mod utils;
mod files;
mod console;
mod system;
mod crypto;
mod time;
mod env_vars;
mod network;
mod media;

use wasmtime::*;
use crate::state::HostState;

fn main() -> Result<()> {
    println!("[Runtime] Запуск...");

    let engine = Engine::default();
    let mut store = Store::new(&engine, HostState::new());
    let mut linker = Linker::new(&engine);

    files::register_files_api(&mut linker)?;
    console::register_console_api(&mut linker)?;
    system::register_system_api(&mut linker)?;
    crypto::register_crypto_api(&mut linker)?;
    time::register_time_api(&mut linker)?;
    env_vars::register_env_api(&mut linker)?;
    network::register_network_api(&mut linker)?;
    media::register_media_api(&mut linker)?;

//     println!("[Runtime] Зарегистрированные хост-функции:");
//
//     // 1. Сначала собираем список (кортеж копируется, так как name/module — это ссылки &str)
//     let items: Vec<(&str, &str, Extern)> = linker.iter(&mut store).collect();
//
//     // 2. Теперь итератор отпустил store, и мы спокойно читаем типы
//     for (module, name, item) in items {
//         println!("  - {} :: {} -> {:?}", module, name, item.ty(&store));
//     }
//
//     // 1. Потоки и Воркеры
//     host_spawn_thread(entry_point, arg_ptr) -> thread_id
//     host_join_thread(thread_id)
//     host_yield()
//
//     // 2. Асинхронные Таски (Event Loop)
//     host_task_spawn(future_fn) -> task_id
//     host_task_poll(task_id) -> status (Pending / Ready)
//     host_task_cancel(task_id)
//
//     // 3. Межпроцессное взаимодействие (IPC / Channels)
//     host_channel_create() -> (tx_id, rx_id)
//     host_channel_send(tx_id, data_ptr, len)
//     host_channel_recv(rx_id, out_ptr)

    let module = Module::from_file(&engine, "app.wasm")?;
    let instance = linker.instantiate(&mut store, &module)?;
    let start_func = instance.get_typed_func::<(), ()>(&mut store, "start")?;

    println!("[Runtime] Вызов функции из WASM:");

        if let Err(err) = start_func.call(&mut store, ()) {
            let err_msg = format!("{:#}", err); // Берем полную цепочку ошибок

            if err_msg.contains("WASM_EXIT_SUCCESS") {
                // Тихо и чистенько завершаем — модуль вывел code: 0
            } else if err_msg.contains("WASM_EXIT_FAILURE") {
                let code = err_msg.split("WASM_EXIT_FAILURE: ")
                    .nth(1)
                    .unwrap_or("неизвестно")
                    .lines()
                    .next()
                    .unwrap_or("");
                eprintln!("[Runtime] Модуль завершил работу с кодом ошибки: {}", code);
            } else {
                // Настоящая ошибка рантайма или паника в WASM
                eprintln!("[Runtime Error] {}", err);
            }
        }

    Ok(())
}

