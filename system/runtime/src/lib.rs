use std::error::Error;
use std::sync::Arc;

use parking_lot::Mutex;

use wasi_common::{
    sync::{add_to_linker, WasiCtxBuilder},
    WasiCtx,
};
use wasmtime::{
    AsContext, AsContextMut, Caller, Engine, Extern, Linker, Memory, MemoryType, Module, Store,
};

fn read_wasm_mem(caller: &mut Caller<'_, WasiCtx>, ptr: i32, len: i32) -> Option<Vec<u8>> {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return None,
    };

    let mut buf = vec![0u8; len as usize];

    match mem.read(caller.as_context_mut(), ptr as usize, &mut buf) {
        Ok(_) => Some(buf),
        Err(_) => None,
    }
}

fn write_wasm_mem(caller: &mut Caller<'_, WasiCtx>, bytes: &[u8]) -> Option<i64> {
    let alloc = match caller.get_export("alloc") {
        Some(Extern::Func(malloc)) => match malloc.typed::<i32, i32>(caller.as_context()) {
            Ok(malloc) => malloc,
            Err(_) => return None,
        },
        _ => return None,
    };

    let len = bytes.len();

    let ptr = match alloc.call(caller.as_context_mut(), len as i32) {
        Ok(ptr) => ptr,
        _ => return None,
    };

    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return None,
    };

    match mem.write(caller.as_context_mut(), ptr as usize, bytes) {
        Ok(_) => {}
        Err(_) => return None,
    };

    let ptr64 = (ptr as i64) << 32;
    let len64 = len as i64;
    Some(ptr64 | len64)
}

pub fn compile_module(src: &[u8], engine: &Engine) -> Result<Module, Box<dyn Error>> {
    let module = Module::new(engine, src)?;
    Ok(module)
}

pub fn invoke_module(
    engine: &Engine,
    module: &Module,
    args: &Option<Vec<u8>>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut linker = Linker::new(engine);
    add_to_linker(&mut linker, |s| s)?;

    let wasi = WasiCtxBuilder::new().inherit_stdio().build();

    let mut store = Store::new(engine, wasi);

    let memory_ty = MemoryType::new(1, None);
    Memory::new(&mut store, memory_ty)?;

    let args = args.clone();

    linker.func_wrap(
        "env",
        "host_get_input",
        move |mut caller: Caller<'_, WasiCtx>| -> i64 {
            if let Some(args) = args.clone() {
                if let Some(res) = write_wasm_mem(&mut caller, &args) {
                    return res;
                }
            }

            0
        },
    )?;

    let out = Arc::new(Mutex::new(vec![]));
    let out_clone = Arc::clone(&out);

    linker.func_wrap(
        "env",
        "host_set_output",
        move |mut caller: Caller<'_, WasiCtx>, ptr: i32, len: i32| -> i32 {
            if let Some(buf) = read_wasm_mem(&mut caller, ptr, len) {
                let mut out = out_clone.lock();
                *out = buf;

                return 1;
            }

            0
        },
    )?;

    // TODO: swap these out so they can be implemented for core
    // ##START_HOST_FNS##
    // linker.func_wrap(
    //     "env",
    //     "host_get_examples_by_pk",
    //     move |mut caller: Caller<'_, WasiCtx>, ptr: i32, len: i32| -> i64 {
    //         if let Some(index_query_bytes) = read_wasm_mem(&mut caller, ptr, len) {
    //             let index_query = match bicycle_proto::IndexQuery::decode(&index_query_bytes[..]) {
    //                 Ok(index_query) => index_query,
    //                 Err(_) => return 0,
    //             };

    //             let examples = match super::get_examples_by_pk(index_query) {
    //                 Ok(examples) => examples,
    //                 Err(_) => return 0,
    //             };

    //             let encoded_examples = bicycle_proto::Examples { examples }.encode_to_vec();

    //             if let Some(res) = write_wasm_mem(&mut caller, &encoded_examples) {
    //                 return res;
    //             }
    //         }

    //         0
    //     },
    // )?;

    // linker.func_wrap(
    //     "env",
    //     "host_delete_examples_by_pk",
    //     move |mut caller: Caller<'_, WasiCtx>, ptr: i32, len: i32| -> i32 {
    //         if let Some(index_query_bytes) = read_wasm_mem(&mut caller, ptr, len) {
    //             let index_query = match bicycle_proto::IndexQuery::decode(&index_query_bytes[..]) {
    //                 Ok(index_query) => index_query,
    //                 Err(_) => return 0,
    //             };

    //             match super::delete_examples_by_pk(index_query) {
    //                 Ok(_) => 1,
    //                 Err(_) => 0,
    //             }
    //         } else {
    //             0
    //         }
    //     },
    // )?;

    // linker.func_wrap(
    //     "env",
    //     "host_put_example",
    //     move |mut caller: Caller<'_, WasiCtx>, ptr: i32, len: i32| -> i32 {
    //         if let Some(example_as_bytes) = read_wasm_mem(&mut caller, ptr, len) {
    //             let example = match bicycle_proto::Example::decode(&example_as_bytes[..]) {
    //                 Ok(example) => example,
    //                 Err(_) => return 0,
    //             };

    //             match super::put_example(example) {
    //                 Ok(_) => 1,
    //                 Err(_) => 0,
    //             }
    //         } else {
    //             0
    //         }
    //     },
    // )?;

    // linker.func_wrap(
    //     "env",
    //     "host_batch_put_examples",
    //     move |mut caller: Caller<'_, WasiCtx>, ptr: i32, len: i32| {
    //         if let Some(examples_as_bytes) = read_wasm_mem(&mut caller, ptr, len) {
    //             let examples = match bicycle_proto::Examples::decode(&examples_as_bytes[..]) {
    //                 Ok(examples) => examples,
    //                 Err(_) => return 0,
    //             };

    //             match super::batch_put_examples(examples) {
    //                 Ok(_) => 1,
    //                 Err(_) => 0,
    //             }
    //         } else {
    //             0
    //         }
    //     },
    // )?;
    // ##END_HOST_FNS##

    linker.module(&mut store, "", module)?;

    linker
        .get_default(&mut store, "")?
        .typed::<(), ()>(&store)?
        .call(&mut store, ())?;

    let out = out.lock();
    Ok(out.clone())
}
