use rand::Rng;
use wasmtime::*;

pub fn register_crypto_api(linker: &mut Linker<()>) -> Result<()> {
    linker.func_wrap("env", "host_random_u32", |_caller: Caller<'_, ()>| -> u32 {
        let mut rng = rand::thread_rng();
        rng.gen::<u32>()
    })?;

    Ok(())
}