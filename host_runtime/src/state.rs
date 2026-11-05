use std::collections::HashMap;
use std::fs::File;
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct HostState {
    pub files: HashMap<i32, File>,
    pub next_fd: i32,

    pub next_net_fd: i32,
    pub tcp_streams: HashMap<i32, TcpStream>,
    pub tcp_listeners: HashMap<i32, TcpListener>,
    pub udp_sockets: HashMap<i32, UdpSocket>,
}

impl HostState {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            next_fd: 3,
            next_net_fd: 1000, // подняли
            tcp_streams: HashMap::new(),
            tcp_listeners: HashMap::new(),
            udp_sockets: HashMap::new(),
        }
    }
}

pub type SharedState = Arc<Mutex<HostState>>;