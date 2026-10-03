// src/state.rs (или прямо в main.rs)
use std::collections::HashMap;
use std::fs::File;
use std::net::{TcpListener, TcpStream, UdpSocket};

#[derive(Default)]
pub struct HostState {
    // Хэндлы файлов
    pub files: HashMap<i32, File>,
    pub next_fd: i32,

    // Сюда же потом добавятся сокеты, процессы и т.д.:
    // pub sockets: HashMap<i32, TcpStream>,

    pub next_net_fd: i32,
    pub tcp_streams: HashMap<i32, TcpStream>,
    pub tcp_listeners: HashMap<i32, TcpListener>,
    pub udp_sockets: HashMap<i32, UdpSocket>,
}

impl HostState {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            next_fd: 3, // 0, 1, 2 зарезервированы под stdin/stdout/stderr
            next_net_fd: 100, // сокеты стартуют, например, со 100
            tcp_streams: HashMap::new(),
            tcp_listeners: HashMap::new(),
            udp_sockets: HashMap::new(),
        }
    }
}