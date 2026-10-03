use wasmtime::*;
use crate::state::HostState;
use std::io::{Read, Write};
use crate::utils::{read_str_from_mem, read_bytes_from_mem, with_mem_slice_mut, write_bytes_to_mem};
use std::net::{TcpStream, TcpListener, UdpSocket};

pub fn register_network_api(linker: &mut Linker<HostState>) -> Result<()> {
    linker.func_wrap(
        "env",
        "host_http_request",
        |mut caller: Caller<'_, HostState>,
         method_ptr: i32, method_len: i32,
         url_ptr: i32, url_len: i32,
         body_ptr: i32, body_len: i32,
         out_ptr: i32, max_len: i32,
         status_out_ptr: i32| -> i32 {

            let Ok(method) = read_str_from_mem(&mut caller, method_ptr, method_len) else { return -1; };
            let Ok(url) = read_str_from_mem(&mut caller, url_ptr, url_len) else { return -1; };

            let body_bytes = if body_len > 0 {
                let Ok(body_str) = read_str_from_mem(&mut caller, body_ptr, body_len) else { return -1; };
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
                Err(_) => return -1,
            };

            let status_bytes = status.to_le_bytes();
            if write_bytes_to_mem(&mut caller, status_out_ptr, &status_bytes).is_err() {
                return -1;
            }

            // Читаем из сокета прямо в WASM-память без лишнего промежуточного vec![0u8; N]
            let read_result = with_mem_slice_mut(&mut caller, out_ptr, max_len, |buf| {
                reader.read(buf)
            });

            match read_result {
                Ok(Ok(n)) => n as i32,
                _ => -1,
            }
        },
    )?;

    // --- TCP CONNECT ---
    linker.func_wrap(
        "env",
        "host_tcp_connect",
        |mut caller: Caller<'_, HostState>, addr_ptr: i32, addr_len: i32| -> i32 {
            let Ok(addr) = read_str_from_mem(&mut caller, addr_ptr, addr_len) else { return -1; };
            let Ok(stream) = TcpStream::connect(addr) else { return -1; };

            let state = caller.data_mut();
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
        |mut caller: Caller<'_, HostState>, addr_ptr: i32, addr_len: i32| -> i32 {
            let Ok(addr) = read_str_from_mem(&mut caller, addr_ptr, addr_len) else { return -1; };
            let Ok(listener) = TcpListener::bind(addr) else { return -1; };

            let state = caller.data_mut();
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
        |mut caller: Caller<'_, HostState>, server_fd: i32| -> i32 {
            let state = caller.data_mut();
            let Some(listener) = state.tcp_listeners.get(&server_fd) else { return -1; };

            let Ok((stream, _)) = listener.accept() else { return -1; };

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
        |mut caller: Caller<'_, HostState>, fd: i32, data_ptr: i32, data_len: i32| -> i32 {
            let Ok(bytes) = read_bytes_from_mem(&mut caller, data_ptr, data_len) else { return -1; };
            let state = caller.data_mut();
            let Some(stream) = state.tcp_streams.get_mut(&fd) else { return -1; };

            stream.write(&bytes).map(|n| n as i32).unwrap_or(-1)
        },
    )?;

    // --- TCP RECV ---
    linker.func_wrap(
        "env",
        "host_tcp_recv",
        |mut caller: Caller<'_, HostState>, fd: i32, buf_ptr: i32, max_len: i32| -> i32 {
            let mut stream = match caller.data_mut().tcp_streams.remove(&fd) {
                Some(s) => s,
                None => return -1,
            };

            let read_res = with_mem_slice_mut(&mut caller, buf_ptr, max_len, |buf| {
                stream.read(buf)
            });

            let bytes_read = match read_res {
                Ok(Ok(n)) => n as i32,
                _ => -1,
            };

            caller.data_mut().tcp_streams.insert(fd, stream);
            bytes_read
        },
    )?;

    // --- TCP CLOSE ---
    linker.func_wrap(
        "env",
        "host_tcp_close",
        |mut caller: Caller<'_, HostState>, fd: i32| -> i32 {
            let state = caller.data_mut();
            if state.tcp_streams.remove(&fd).is_some() || state.tcp_listeners.remove(&fd).is_some() {
                0
            } else {
                -1
            }
        },
    )?;

    // --- UDP BIND ---
    linker.func_wrap(
        "env",
        "host_udp_bind",
        |mut caller: Caller<'_, HostState>, addr_ptr: i32, addr_len: i32| -> i32 {
            let Ok(addr) = read_str_from_mem(&mut caller, addr_ptr, addr_len) else { return -1; };
            let Ok(sock) = UdpSocket::bind(addr) else { return -1; };

            let state = caller.data_mut();
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
        |mut caller: Caller<'_, HostState>, fd: i32, data_ptr: i32, data_len: i32, target_ptr: i32, target_len: i32| -> i32 {
            let Ok(bytes) = read_bytes_from_mem(&mut caller, data_ptr, data_len) else { return -1; };
            let Ok(target) = read_str_from_mem(&mut caller, target_ptr, target_len) else { return -1; };

            let state = caller.data_mut();
            let Some(sock) = state.udp_sockets.get(&fd) else { return -1; };

            sock.send_to(&bytes, target).map(|n| n as i32).unwrap_or(-1)
        },
    )?;

    // --- UDP RECV FROM ---
    linker.func_wrap(
        "env",
        "host_udp_recv_from",
        |mut caller: Caller<'_, HostState>, fd: i32, buf_ptr: i32, max_len: i32, sender_ptr: i32, sender_max_len: i32| -> i32 {
            let sock = match caller.data_mut().udp_sockets.remove(&fd) {
                Some(s) => s,
                None => return -1,
            };

            let mut temp_buf = vec![0u8; max_len as usize];
            match sock.recv_from(&mut temp_buf) {
                Ok((n, src)) => {
                    caller.data_mut().udp_sockets.insert(fd, sock);

                    // Записываем полученные данные в буфер WASM
                    if write_bytes_to_mem(&mut caller, buf_ptr, &temp_buf[..n]).is_err() {
                        return -1;
                    }

                    // Записываем адрес отправителя в буфер
                    let src_bytes = src.to_string().into_bytes();
                    if src_bytes.len() <= sender_max_len as usize {
                        let _ = write_bytes_to_mem(&mut caller, sender_ptr, &src_bytes);
                    }

                    // Упаковываем: верхние 16 бит — длина адреса, нижние 16 — кол-во байт
                    ((src_bytes.len() as i32) << 16) | (n as i32 & 0xFFFF)
                }
                Err(_) => {
                    caller.data_mut().udp_sockets.insert(fd, sock);
                    -1
                }
            }
        },
    )?;

    // --- UDP CLOSE ---
    linker.func_wrap(
        "env",
        "host_udp_close",
        |mut caller: Caller<'_, HostState>, fd: i32| -> i32 {
            if caller.data_mut().udp_sockets.remove(&fd).is_some() { 0 } else { -1 }
        },
    )?;

    Ok(())
}