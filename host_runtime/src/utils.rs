use wasmtime::*;
use crate::state::HostState;

pub fn read_str_from_mem(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Option<String> {
    let memory = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return None,
    };
    let data = memory.data(caller);
    let start = ptr as usize;
    let end = start + len as usize;

    let slice = data.get(start..end)?;
    std::str::from_utf8(slice).ok().map(|s| s.to_string())
}

// original function
// fn get_path_from_mem(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Option<String> {
//     let memory = match caller.get_export("memory") {
//         Some(Extern::Memory(mem)) => mem,
//         _ => return None,
//     };
//     let data = memory.data(caller);
//     let slice = data.get(ptr as usize..(ptr + len) as usize)?;
//     std::str::from_utf8(slice).ok().map(|s| s.to_string())
// }