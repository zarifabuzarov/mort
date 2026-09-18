// src/state.rs (или прямо в main.rs)
use std::collections::HashMap;
use std::fs::File;

#[derive(Default)]
pub struct HostState {
    // Хэндлы файлов
    pub files: HashMap<i32, File>,
    pub next_fd: i32,

    // Сюда же потом добавятся сокеты, процессы и т.д.:
    // pub sockets: HashMap<i32, TcpStream>,
}

impl HostState {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            next_fd: 3, // 0, 1, 2 зарезервированы под stdin/stdout/stderr
        }
    }
}