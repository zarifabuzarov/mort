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
use crate::state::{HostState, SharedState};
use std::sync::{Arc, Mutex};


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

#[tokio::main]
async fn main() -> Result<()> {
    println!("[Runtime] Запуск...");

    let engine = Engine::default();
    let shared_state: SharedState = Arc::new(Mutex::new(HostState::new()));

    let mut handles = vec![];

    for i in 0..3 {
        let engine = engine.clone();
        let shared_state = shared_state.clone();

        let handle = tokio::spawn(async move {
            println!("[Runtime] Запуск модуля #{}", i);

            // Store должен жить в одном потоке/задаче
            let mut store = Store::new(&engine, shared_state.clone());
            let mut linker = Linker::new(&engine);

            files::register_files_api(&mut linker).unwrap();
            console::register_console_api(&mut linker).unwrap();
            system::register_system_api(&mut linker).unwrap();
            crypto::register_crypto_api(&mut linker).unwrap();
            time::register_time_api(&mut linker).unwrap();
            env_vars::register_env_api(&mut linker).unwrap();
            network::register_network_api(&mut linker).unwrap();
            media::register_media_api(&mut linker).unwrap();

            let module = Module::from_file(&engine, "app.wasm").unwrap();
            let instance = linker.instantiate(&mut store, &module).unwrap();
            let start_func = instance
                .get_typed_func::<(), ()>(&mut store, "start")
                .unwrap();

            println!("[Runtime] Модуль #{} вызывает start()", i);

            // Пока синхронный вызов. Позже можно сделать async host functions.
            if let Err(err) = start_func.call(&mut store, ()) {
                let err_msg = format!("{:#}", err);
                if err_msg.contains("WASM_EXIT_SUCCESS") {
                    println!("[Runtime] Модуль #{} завершился успешно", i);
                } else if err_msg.contains("WASM_EXIT_FAILURE") {
                    eprintln!("[Runtime] Модуль #{} завершился с ошибкой", i);
                } else {
                    eprintln!("[Runtime Error] Модуль #{}: {}", i, err);
                }
            }
        });

        handles.push(handle);
    }

    // Ждём все задачи
    for handle in handles {
        handle.await.unwrap();
    }

    println!("[Runtime] Все модули завершились");
    Ok(())
}