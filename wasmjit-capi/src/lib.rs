#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(clippy::missing_safety_doc, clippy::new_without_default)
)]

use std::mem;
use std::ptr;
use std::slice;
use std::sync::Arc;

use ontio_wasmjit::chain_api::ChainCtx;
use ontio_wasmjit::chain_api::{Address, ChainResolver};
use ontio_wasmjit::executor::build_module;
use ontio_wasmjit::resolver::Resolver;
use ontio_wasmjit_runtime::{ExecMetrics, VMContext};

use cranelift_wasm::DefinedMemoryIndex;
use ontio_wasm_build::wasm_validate;
use ontio_wasmjit::error::Error;
use ontio_wasmjit::executor::{Instance, Module};
pub use ontio_wasmjit_runtime::builtins::{
    check_internel_panic, wasmjit_result_err_compile, wasmjit_result_err_internal,
    wasmjit_result_err_link, wasmjit_result_err_trap, wasmjit_result_kind, wasmjit_result_success,
};
use std::fs::File;
use std::io::Read;

#[repr(C)]
pub struct wasmjit_result_t {
    pub kind: wasmjit_result_kind,
    pub msg: wasmjit_bytes_t,
}

#[repr(C)]
pub struct wasmjit_bytes_t {
    pub data: *mut u8,
    pub len: u32,
}

pub fn bytes_null() -> wasmjit_bytes_t {
    wasmjit_bytes_t {
        data: std::ptr::null_mut(),
        len: 0,
    }
}

pub fn bytes_from_vec(data: Vec<u8>) -> wasmjit_bytes_t {
    let bytes: Box<[u8]> = data.into_boxed_slice();
    let len = bytes.len() as u32;
    let data = Box::into_raw(bytes) as *mut u8;
    wasmjit_bytes_t { data, len }
}

pub unsafe fn bytes_to_boxed_slice(bytes: wasmjit_bytes_t) -> Box<[u8]> {
    let raw = slice::from_raw_parts_mut(bytes.data, bytes.len as usize);
    Box::from_raw(raw)
}

unsafe fn slice_to_ref<'a>(s: wasmjit_slice_t) -> &'a [u8] {
    slice::from_raw_parts(s.data, s.len as usize)
}

#[no_mangle]
pub extern "C" fn wasmjit_bytes_new(len: u32) -> wasmjit_bytes_t {
    bytes_from_vec(vec![0; len as usize])
}

#[no_mangle]
pub extern "C" fn wasmjit_bytes_as_slice(bytes: wasmjit_bytes_t) -> wasmjit_slice_t {
    wasmjit_slice_t {
        data: bytes.data,
        len: bytes.len,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_bytes_destroy(bytes: wasmjit_bytes_t) {
    drop(bytes_to_boxed_slice(bytes));
}

#[derive(Debug)]
#[repr(C)]
pub struct wasmjit_slice_t {
    pub data: *mut u8,
    pub len: u32,
}

#[repr(C)]
pub struct wasmjit_resolver_t {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct wasmjit_instance_t {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct wasmjit_vmctx_t {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct wasmjit_module_t {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct wasmjit_chain_context_t {
    _unused: [u8; 0],
}

pub type h256_t = [u8; 32];

pub type address_t = [u8; 20];

unsafe fn addrs_from_slice(callers: wasmjit_slice_t) -> Vec<Address> {
    let buf = slice::from_raw_parts(callers.data, callers.len as usize);
    let mut callers = Vec::with_capacity(callers.len as usize / 20);

    for addr in buf.chunks_exact(20) {
        let mut caller = [0; 20];
        caller[0..].copy_from_slice(addr);
        callers.push(caller);
    }

    callers
}

pub unsafe fn convert_vmctx<'a>(ctx: *mut wasmjit_vmctx_t) -> &'a mut VMContext {
    &mut *(ctx as *mut VMContext)
}

/// Implementation of wasmjit_vmctx_chainctx
#[no_mangle]
pub unsafe extern "C" fn wasmjit_vmctx_chainctx(
    vmctx: *mut wasmjit_vmctx_t,
) -> *mut wasmjit_chain_context_t {
    let vmctx_r = convert_vmctx(vmctx);
    let host = (&mut *vmctx_r).host_state();
    host.downcast_mut::<ChainCtx>().unwrap() as *mut ChainCtx as *mut wasmjit_chain_context_t
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_vmctx_memory(
    ctx: *mut wasmjit_vmctx_t,
    result: &mut wasmjit_slice_t,
) -> wasmjit_result_t {
    let ctx = convert_vmctx(ctx);
    let mem = ctx
        .instance()
        .memory_slice_mut(DefinedMemoryIndex::from_u32(0));
    match mem {
        Some(mem) => {
            result.data = mem as *mut [u8] as *mut u8;
            result.len = mem.len() as u32;
            wasmjit_result_t {
                kind: wasmjit_result_success,
                msg: bytes_null(),
            }
        }
        None => wasmjit_result_t {
            kind: wasmjit_result_err_trap,
            msg: bytes_from_vec(b"undefined memory".to_vec()),
        },
    }
}

pub type u8x6 = [u8; 4];

#[no_mangle]
pub extern "C" fn abi_test(a1: u32, a2: u32, a3: u64, a4: u64, a5: u64, a6: u64, a7: &u8x6) {
    println!("args: {:?}", (a1, a2, a3, a4, a5, a6, a7))
}

#[no_mangle]
pub extern "C" fn wasmjit_chain_context_create(
    height: u32,
    blockhash: &mut h256_t,
    timestamp: u64,
    txhash: &mut h256_t,
    callers_raw: wasmjit_slice_t,
    witness_raw: wasmjit_slice_t,
    input_raw: wasmjit_slice_t,
    exec_step: u64,
    gas_factor: u64,
    gas_left: u64,
    depth_left: u64,
    service_index: u64,
) -> *mut wasmjit_chain_context_t {
    assert_eq!(callers_raw.len % 20, 0);
    assert_eq!(witness_raw.len % 20, 0);

    let (callers, witness, input) = unsafe {
        (
            addrs_from_slice(callers_raw),
            addrs_from_slice(witness_raw),
            slice::from_raw_parts(input_raw.data, input_raw.len as usize).to_vec(),
        )
    };

    let exec_metrics = ExecMetrics::new(exec_step, gas_factor, gas_left, depth_left);
    let ctx = ChainCtx::new(
        timestamp,
        height,
        *blockhash,
        *txhash,
        callers,
        witness,
        input,
        exec_metrics,
        service_index,
    );

    Box::into_raw(Box::new(ctx)) as *mut wasmjit_chain_context_t
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_chain_context_destroy(ctx: *mut wasmjit_chain_context_t) {
    drop(Box::from_raw(ctx as *mut ChainCtx));
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_chain_context_push_caller(
    ctx: *mut wasmjit_chain_context_t,
    caller: &address_t,
) {
    let ctx = convert_chain_ctx(ctx);
    ctx.push_caller(*caller);
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_chain_context_pop_caller(
    ctx: *mut wasmjit_chain_context_t,
    result: &mut address_t,
) {
    let ctx = convert_chain_ctx(ctx);
    *result = ctx.pop_caller().unwrap_or([0; 20]);
}

pub unsafe fn convert_chain_ctx<'a>(ctx: *mut wasmjit_chain_context_t) -> &'a mut ChainCtx {
    &mut *(ctx as *mut ChainCtx)
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_chain_context_get_gas(ctx: *mut wasmjit_chain_context_t) -> u64 {
    let ctx = convert_chain_ctx(ctx);
    ctx.gas_left()
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_chain_context_set_gas(
    ctx: *mut wasmjit_chain_context_t,
    gas: u64,
) {
    let ctx = convert_chain_ctx(ctx);
    ctx.set_gas_left(gas);
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_chain_context_get_exec_step(
    ctx: *mut wasmjit_chain_context_t,
) -> u64 {
    let ctx = convert_chain_ctx(ctx);
    ctx.exec_step()
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_chain_context_set_exec_step(
    ctx: *mut wasmjit_chain_context_t,
    exec_step: u64,
) {
    let ctx = convert_chain_ctx(ctx);
    ctx.set_exec_step(exec_step);
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_chain_context_set_calloutput(
    ctx: *mut wasmjit_chain_context_t,
    bytes: wasmjit_bytes_t,
) {
    let ctx = convert_chain_ctx(ctx);
    ctx.set_calloutput(bytes_to_boxed_slice(bytes).to_vec());
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_chain_context_take_output(
    ctx: *mut wasmjit_chain_context_t,
) -> wasmjit_bytes_t {
    let ctx = convert_chain_ctx(ctx);
    bytes_from_vec(ctx.take_output())
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_compile(
    compiled: &mut *mut wasmjit_module_t,
    wasm: wasmjit_slice_t,
) -> wasmjit_result_t {
    let wasm = slice_to_ref(wasm);

    let panic = check_internel_panic(|| Ok(build_module(wasm)));

    let result = match panic {
        Ok(res) => res,
        Err(msg) => {
            return wasmjit_result_t {
                kind: wasmjit_result_err_internal,
                msg: bytes_from_vec(msg.into_bytes()),
            }
        }
    };

    match result {
        Ok(module) => {
            *compiled = Arc::into_raw(module) as *mut wasmjit_module_t;
            wasmjit_result_t {
                kind: wasmjit_result_success,
                msg: bytes_null(),
            }
        }
        Err(error) => result_from_error(error),
    }
}

fn result_from_error(error: Error) -> wasmjit_result_t {
    match error {
        Error::Compile(comp) => wasmjit_result_t {
            kind: wasmjit_result_err_compile,
            msg: bytes_from_vec(comp.to_string().into_bytes()),
        },
        Error::Link(link) => wasmjit_result_t {
            kind: wasmjit_result_err_link,
            msg: bytes_from_vec(link.into_bytes()),
        },
        Error::Internal(intern) => wasmjit_result_t {
            kind: wasmjit_result_err_internal,
            msg: bytes_from_vec(intern.into_bytes()),
        },
        Error::Trap(trap) => wasmjit_result_t {
            kind: wasmjit_result_err_trap,
            msg: bytes_from_vec(trap.into_bytes()),
        },
    }
}

unsafe fn module_ref_to_impl_repr(module: *const wasmjit_module_t) -> Arc<Module> {
    let module = Arc::from_raw(module as *const Module);
    mem::forget(module.clone());

    module
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_module_destroy(module: *mut wasmjit_module_t) {
    drop(Arc::from_raw(module as *const Module));
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_module_instantiate(
    module: *const wasmjit_module_t,
    resolver: *mut wasmjit_resolver_t,
    instance: &mut *mut wasmjit_instance_t,
) -> wasmjit_result_t {
    let module = module_ref_to_impl_repr(module);
    let mut resolver = resolver_to_impl_repr(resolver);

    let panic = check_internel_panic(|| Ok(module.instantiate(&mut **resolver)));
    let result = match panic {
        Ok(res) => res,
        Err(msg) => {
            return wasmjit_result_t {
                kind: wasmjit_result_err_internal,
                msg: bytes_from_vec(msg.into_bytes()),
            }
        }
    };

    match result {
        Ok(inst) => {
            let inst = Box::new(inst);
            *instance = Box::into_raw(inst) as *mut wasmjit_instance_t;
            wasmjit_result_t {
                kind: wasmjit_result_success,
                msg: bytes_null(),
            }
        }
        Err(error) => result_from_error(error),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_instantiate(
    instance: &mut *mut wasmjit_instance_t,
    resolver: *mut wasmjit_resolver_t,
    wasm: wasmjit_slice_t,
) -> wasmjit_result_t {
    let mut compiled: *mut wasmjit_module_t = ptr::null_mut();
    let result = wasmjit_compile(&mut compiled, wasm);
    if result.kind != wasmjit_result_success {
        return result;
    }
    let result =
        wasmjit_module_instantiate(compiled as *const wasmjit_module_t, resolver, instance);
    wasmjit_module_destroy(compiled);

    result
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_instance_invoke(
    instance: *mut wasmjit_instance_t,
    ctx: *mut wasmjit_chain_context_t,
) -> wasmjit_result_t {
    let inst = &mut *(instance as *mut Instance);
    let cctx = Box::from_raw(ctx as *mut ChainCtx);

    let panic = check_internel_panic(|| Ok(inst.invoke(cctx)));
    let result = match panic {
        Ok(res) => res,
        Err(msg) => {
            return wasmjit_result_t {
                kind: wasmjit_result_err_internal,
                msg: bytes_from_vec(msg.into_bytes()),
            }
        }
    };

    match result {
        Ok(_) => wasmjit_result_t {
            kind: wasmjit_result_success,
            msg: bytes_null(),
        },
        Err(err) => result_from_error(err),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_instance_destroy(instance: *mut wasmjit_instance_t) {
    drop(Box::from_raw(instance as *mut Instance));
}

unsafe fn resolver_to_impl_repr(resolver: *mut wasmjit_resolver_t) -> Box<Box<dyn Resolver>> {
    let resolver = resolver as *mut Box<dyn Resolver>;
    Box::from_raw(resolver)
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_resolver_destroy(resolver: *mut wasmjit_resolver_t) {
    let resolver = resolver as *mut Box<dyn Resolver>;
    let _ = Box::from_raw(resolver);
}

#[no_mangle]
pub extern "C" fn wasmjit_simple_resolver_create() -> *mut wasmjit_resolver_t {
    let res = ChainResolver;
    let b1 = Box::new(res) as Box<dyn Resolver>;

    Box::into_raw(Box::new(b1)) as *mut wasmjit_resolver_t
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_validate(wasm: wasmjit_slice_t) -> wasmjit_result_t {
    let wasm = slice_to_ref(wasm);
    let result = wasm_validate(wasm);
    match result {
        Ok(_) => wasmjit_result_t {
            kind: wasmjit_result_success,
            msg: bytes_null(),
        },
        Err(error) => wasmjit_result_t {
            kind: wasmjit_result_err_compile,
            msg: bytes_from_vec(error.to_string().into_bytes()),
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmjit_test_read_wasm_file(name: wasmjit_slice_t) -> wasmjit_bytes_t {
    let fpath = slice_to_ref(name);
    let fpath_str = String::from_utf8(fpath.to_vec()).expect("invalid file name");
    let mut file = File::open(&fpath_str).expect("couldn't open");
    let mut s = String::new();
    file.read_to_string(&mut s).expect("read file error");
    let wasm = wast::parse_str(&s).unwrap();
    bytes_from_vec(wasm)
}
