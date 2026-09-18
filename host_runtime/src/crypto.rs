use rand::Rng;
use wasmtime::*;
use crate::state::HostState;

pub fn register_crypto_api(linker: &mut Linker<HostState>) -> Result<()> {
    linker.func_wrap("env", "host_random_u32", |_caller: Caller<'_, HostState>| -> u32 {
        let mut rng = rand::thread_rng();
        rng.gen::<u32>()
    })?;

    Ok(())
}