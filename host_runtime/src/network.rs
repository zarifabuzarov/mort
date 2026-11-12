use wasmtime::*;
use crate::state::SharedState;
use std::io::{Read, Write};
use crate::utils::{read_str_from_mem, read_bytes_from_mem, with_mem_slice_mut, write_bytes_to_mem};
use std::net::{TcpStream, TcpListener, UdpSocket};

pub fn register_network_api(linker: &mut Linker<SharedState>) -> Result<()> {
    linker.func_wrap(
        "env",
        "host_http_request",
        |mut caller: Caller<'_, SharedState>,
         method_ptr: i32, method_len: i32,
         url_ptr: i32, url_len: i32,
         body_ptr: i32, body_len: i32,
         out_ptr: i32, max_len: i32,
         status_out_ptr: i32| -> i32 {

            let method = match read_str_from_mem(&mut caller, method_ptr, method_len) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("host_http_request: method: {e}");
                    return -1;
                }
            };
            let url = match read_str_from_mem(&mut caller, url_ptr, url_len) {
                Ok(u) => u,
                Err(e) => {
                    eprintln!("host_http_request: url: {e}");
                    return -1;
                }
            };

            let body_bytes = if body_len > 0 {
                let body_str = match read_str_from_mem(&mut caller, body_ptr, body_len) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("host_http_request: body: {e}");
                        return -1;
                    }
                };
                body_str.into_bytes()
            } else {
                Vec::new()
            };

            let req = ureq::request(&method.to_uppercase(), &url)
                .timeout(std::time::Duration::from_secs(10));

            let response_result = if body_bytes.is_empty() {
                req.call()
            } else {
                req.send_bytes(&body_bytes)
            };

            let (status, mut reader) = match response_result {
                Ok(resp) => (resp.status() as i32, resp.into_reader()),
                Err(ureq::Error::Status(code, resp)) => (code as i32, resp.into_reader()),
                Err(e) => {
                    eprintln!("host_http_request: request failed: {e}");
                    return -1;
                },
            };

            let status_bytes = status.to_le_bytes();
            if let Err(e) = write_bytes_to_mem(&mut caller, status_out_ptr, &status_bytes) {
                eprintln!("host_http_request: status write: {e}");
                return -1;
            }

            // Читаем из сокета прямо в WASM-память без лишнего промежуточного vec![0u8; N]
            let read_result = with_mem_slice_mut(&mut caller, out_ptr, max_len, |buf| {
                reader.read(buf)
            });

            match read_result {
                Ok(Ok(n)) => n as i32,
                Ok(Err(e)) => {
                    eprintln!("host_http_request: read body: {e}");
                    -1
                }
                Err(e) => {
                    eprintln!("host_http_request: {e}");
                    -1
                }
            }
        },
    )?;

    // --- TCP CONNECT ---
    linker.func_wrap(
        "env",
        "host_tcp_connect",
        |mut caller: Caller<'_, SharedState>, addr_ptr: i32, addr_len: i32| -> i32 {
            let addr = match read_str_from_mem(&mut caller, addr_ptr, addr_len) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("host_tcp_connect: addr: {e}");
                    return -1;
                }
            };
            let stream = match TcpStream::connect(&addr) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("host_tcp_connect: connect to '{addr}': {e}");
                    return -1;
                }
            };

            let mut state = caller.data().lock().unwrap();
            let fd = state.next_net_fd;
            state.next_net_fd += 1;
            state.tcp_streams.insert(fd, stream);
            fd
        },
    )?;

    // --- TCP LISTEN ---
    linker.func_wrap(
        "env",
        "host_tcp_listen",
        |mut caller: Caller<'_, SharedState>, addr_ptr: i32, addr_len: i32| -> i32 {
            let addr = match read_str_from_mem(&mut caller, addr_ptr, addr_len) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("host_tcp_listen: addr: {e}");
                    return -1;
                }
            };
            let listener = match TcpListener::bind(&addr) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("host_tcp_listen: bind '{addr}': {e}");
                    return -1;
                }
            };

            let mut state = caller.data().lock().unwrap();
            let fd = state.next_net_fd;
            state.next_net_fd += 1;
            state.tcp_listeners.insert(fd, listener);
            fd
        },
    )?;

    // --- TCP ACCEPT ---
    linker.func_wrap(
        "env",
        "host_tcp_accept",
        #[allow(unused_mut)]
        |mut caller: Caller<'_, SharedState>, server_fd: i32| -> i32 {
            let mut state = caller.data().lock().unwrap();
            let Some(listener) = state.tcp_listeners.get(&server_fd) else {
                eprintln!("host_tcp_accept: invalid fd {server_fd}");
                return -1;
            };
            let (stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("host_tcp_accept: accept: {e}");
                    return -1;
                }
            };

            let fd = state.next_net_fd;
            state.next_net_fd += 1;
            state.tcp_streams.insert(fd, stream);
            fd
        },
    )?;

    // --- TCP SEND ---
    linker.func_wrap(
        "env",
        "host_tcp_send",
        |mut caller: Caller<'_, SharedState>, fd: i32, data_ptr: i32, data_len: i32| -> i32 {
            let bytes = match read_bytes_from_mem(&mut caller, data_ptr, data_len) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("host_tcp_send: data: {e}");
                    return -1;
                }
            };
            let mut state = caller.data().lock().unwrap();
            let Some(stream) = state.tcp_streams.get_mut(&fd) else {
                eprintln!("host_tcp_send: invalid fd {fd}");
                return -1;
            };
            match stream.write(&bytes) {
                Ok(n) => n as i32,
                Err(e) => {
                    eprintln!("host_tcp_send: write: {e}");
                    -1
                }
            }
        },
    )?;

    // --- TCP RECV ---
    linker.func_wrap(
        "env",
        "host_tcp_recv",
        |mut caller: Caller<'_, SharedState>, fd: i32, buf_ptr: i32, max_len: i32| -> i32 {
            let mut stream = match caller.data_mut().lock().unwrap().tcp_streams.remove(&fd) {
                Some(s) => s,
                None => {
                    eprintln!("host_tcp_recv: invalid fd {fd}");
                    return -1;
                }
            };

            let read_res = with_mem_slice_mut(&mut caller, buf_ptr, max_len, |buf| {
                stream.read(buf)
            });

            // ВСЕГДА возвращаем stream обратно, даже при ошибке
            caller.data_mut().lock().unwrap().tcp_streams.insert(fd, stream);

            match read_res {
                Ok(Ok(n)) => n as i32,
                Ok(Err(e)) => {
                    eprintln!("host_tcp_recv: read: {e}");
                    -1
                }
                Err(e) => {
                    eprintln!("host_tcp_recv: {e}");
                    -1
                }
            }
        },
    )?;

    // --- TCP CLOSE ---
    linker.func_wrap(
        "env",
        "host_tcp_close",
        #[allow(unused_mut)]
        |mut caller: Caller<'_, SharedState>, fd: i32| -> i32 {
            let mut state = caller.data().lock().unwrap();
            if state.tcp_streams.remove(&fd).is_some() || state.tcp_listeners.remove(&fd).is_some() {
                0
            } else {
                eprintln!("host_tcp_close: invalid fd {fd}");
                -1
            }
        },
    )?;

    // --- UDP BIND ---
    linker.func_wrap(
        "env",
        "host_udp_bind",
        |mut caller: Caller<'_, SharedState>, addr_ptr: i32, addr_len: i32| -> i32 {
            let addr = match read_str_from_mem(&mut caller, addr_ptr, addr_len) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("host_udp_bind: addr: {e}");
                    return -1;
                }
            };
            let sock = match UdpSocket::bind(&addr) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("host_udp_bind: bind '{addr}': {e}");
                    return -1;
                }
            };

            let mut state = caller.data().lock().unwrap();
            let fd = state.next_net_fd;
            state.next_net_fd += 1;
            state.udp_sockets.insert(fd, sock);
            fd
        },
    )?;

    // --- UDP SEND TO ---
    linker.func_wrap(
        "env",
        "host_udp_send_to",
        #[allow(unused_mut)]
        |mut caller: Caller<'_, SharedState>, fd: i32, data_ptr: i32, data_len: i32, target_ptr: i32, target_len: i32| -> i32 {

            let bytes = match read_bytes_from_mem(&mut caller, data_ptr, data_len) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("host_udp_send_to: data: {e}");
                    return -1;
                }
            };
            let target = match read_str_from_mem(&mut caller, target_ptr, target_len) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("host_udp_send_to: target: {e}");
                    return -1;
                }
            };
            let mut state = caller.data().lock().unwrap();
            let Some(sock) = state.udp_sockets.get(&fd) else {
                eprintln!("host_udp_send_to: invalid fd {fd}");
                return -1;
            };
            match sock.send_to(&bytes, &target) {
                Ok(n) => n as i32,
                Err(e) => {
                    eprintln!("host_udp_send_to: send_to '{target}': {e}");
                    -1
                }
            }
        },
    )?;

    // --- UDP RECV FROM ---
    linker.func_wrap(
        "env",
        "host_udp_recv_from",
        |mut caller: Caller<'_, SharedState>, fd: i32, buf_ptr: i32, max_len: i32, sender_ptr: i32, sender_max_len: i32| -> i32 {
            let sock = match caller.data_mut().lock().unwrap().udp_sockets.remove(&fd) {
                Some(s) => s,
                None => {
                    eprintln!("host_udp_recv_from: invalid fd {fd}");
                    return -1;
                }
            };

            let mut temp_buf = vec![0u8; max_len as usize];
            match sock.recv_from(&mut temp_buf) {
                Ok((n, src)) => {
                    // Сначала пишем данные в память WASM (sock ещё в руках)
                    if let Err(e) = write_bytes_to_mem(&mut caller, buf_ptr, &temp_buf[..n]) {
                        eprintln!("host_udp_recv_from: write data: {e}");
                        caller.data_mut().lock().unwrap().udp_sockets.insert(fd, sock);
                        return -1;
                    }

                    // Записываем адрес отправителя
                    let src_bytes = src.to_string().into_bytes();
                    if src_bytes.len() <= sender_max_len as usize {
                        if let Err(e) = write_bytes_to_mem(&mut caller, sender_ptr, &src_bytes) {
                            eprintln!("host_udp_recv_from: write sender: {e}");
                        }
                    }

                    // Возвращаем sock обратно ОДИН раз, после всех операций
                    caller.data_mut().lock().unwrap().udp_sockets.insert(fd, sock);

                    ((src_bytes.len() as i32) << 16) | (n as i32 & 0xFFFF)
                }
                Err(e) => {
                    eprintln!("host_udp_recv_from: recv_from: {e}");
                    caller.data_mut().lock().unwrap().udp_sockets.insert(fd, sock);
                    -1
                }
            }
        },
    )?;

    // --- UDP CLOSE ---
    linker.func_wrap(
        "env",
        "host_udp_close",
        |mut caller: Caller<'_, SharedState>, fd: i32| -> i32 {
            if caller.data_mut().lock().unwrap().udp_sockets.remove(&fd).is_some() {
                0
            } else {
                eprintln!("host_udp_close: invalid fd {fd}");
                -1
            }
        },
    )?;

    Ok(())
}