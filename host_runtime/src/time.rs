use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use wasmtime::*;
use crate::state::HostState;

pub fn register_time_api(linker: &mut Linker<HostState>) -> Result<()> {
    // 1. Время Unix в секундах
    linker.func_wrap("env", "host_now_unix", |_caller: Caller<'_, HostState>| -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    })?;

    // 2. Время Unix в миллисекундах
    linker.func_wrap("env", "host_now_millis", |_caller: Caller<'_, HostState>| -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    })?;

    // 3. Монотонные наносекунды (для точных замеров и бенчмарков, не зависят от часов ОС)
    // Используем lazy_static / Instant от старта программы
    linker.func_wrap("env", "host_monotonic_nanos", |_caller: Caller<'_, HostState>| -> u64 {
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let start = START.get_or_init(Instant::now);
        start.elapsed().as_nanos() as u64
    })?;

    // 4. Пауза / Сон (Sleep) в миллисекундах
    linker.func_wrap("env", "host_sleep", |_caller: Caller<'_, HostState>, ms: u64| {
        thread::sleep(Duration::from_millis(ms));
    })?;

    Ok(())
}