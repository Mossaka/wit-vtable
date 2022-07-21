use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Result;
use handler::Event;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{StringArrayError, WasiCtx};
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::*;
use wit_bindgen_wasmtime::rt::RawMem;
wit_bindgen_wasmtime::import!("handler.wit");
wit_bindgen_wasmtime::export!("events.wit");

#[derive(Default)]
pub struct Exec {
    guest_instance: Option<Arc<Mutex<Instance>>>,
    guest_store: Option<Arc<Mutex<Store<GuestContext>>>>,
    vtable: Vec<String>,
}

impl events::Events for Exec {
    type Events = ();

    fn events_new(&mut self) -> Self::Events {}

    fn events_listen(&mut self, _self_: &Self::Events, id: &str) -> Self::Events {
        self.vtable.push(id.to_string());
    }

    fn events_exec(&mut self, self_: &Self::Events, _duration: u64) -> Self::Events {
        let mut thread_handles = vec![];
        for i in 1..4 {
            let store = self.guest_store.as_mut().unwrap().clone();
            let instance = self.guest_instance.as_ref().unwrap().clone();
            let vtable = self.vtable.clone();
            thread_handles.push(thread::spawn(move || {
                let mut store = store.lock().unwrap();
                let instance = instance.lock().unwrap();
                // call handler
                for func_name in &vtable {
                    let ev = Event {
                        id: &format!("call-{}-{}", func_name, i),
                        data: "data",
                    };
                    let mut handler = handler::Handler::new(
                        store.deref_mut(),
                        instance.deref(),
                        |ctx: &mut GuestContext| &mut ctx.host.data,
                    )
                    .unwrap();
                    handler.handler = instance
                        .get_typed_func::<(i32, i32, i32, i32), (), _>(
                            store.deref_mut(),
                            &func_name_to_abi_name(func_name),
                        )
                        .unwrap();
                    handler.handler(store.deref_mut(), ev);
                }
            }));
        }
        for handle in thread_handles {
            handle.join().unwrap();
        }
    }
}

#[derive(Default)]
pub struct GuestExec {
    data: handler::HandlerData,
}

impl events::Events for GuestExec {
    type Events = ();

    fn events_new(&mut self) -> Self::Events {}

    fn events_listen(&mut self, _self_: &Self::Events, _id: &str) -> Self::Events {}

    fn events_exec(&mut self, _self_: &Self::Events, _duration: u64) -> Self::Events {}
}

fn main() -> Result<()> {
    let engine = Engine::new(&default_config()?)?;
    let path = "target/wasm32-wasi/debug/guest.wasm";

    let (mut store, _linker, instance) = wasmtime_init(&engine, path)?;
    let (store2, _linker2, instance2) = wasmtime_init(&engine, path)?;

    store.data_mut().host = Exec {
        guest_instance: Some(Arc::new(Mutex::new(instance2))),
        guest_store: Some(Arc::new(Mutex::new(store2))),
        ..Default::default()
    };
    instance
        .get_typed_func::<(), (), _>(&mut store, "_start")?
        .call(&mut store, ())?;

    Ok(())
}

pub fn default_config() -> Result<Config> {
    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_multi_memory(true);
    config.wasm_module_linking(true);
    Ok(config)
}

pub fn default_wasi() -> Result<WasiCtx, StringArrayError> {
    let mut ctx: WasiCtxBuilder = WasiCtxBuilder::new().inherit_stdio().inherit_args()?;
    ctx = ctx
        .preopened_dir(
            Dir::open_ambient_dir("./target", ambient_authority()).unwrap(),
            "cache",
        )
        .unwrap();

    Ok(ctx.build())
}

pub fn wasmtime_init<T: events::Events + Default>(
    engine: &Engine,
    path: &str,
) -> Result<(Store<Context<T>>, Linker<Context<T>>, Instance)>
where
{
    let ctx = Context::default();
    let mut linker = Linker::new(engine);
    let mut store = Store::new(engine, ctx);
    let module = Module::from_file(engine, path)?;
    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut Context<T>| &mut cx.wasi)?;
    events::add_to_linker(&mut linker, |cx: &mut Context<T>| {
        (&mut cx.host, &mut cx.host_tables)
    })?;
    let instance = linker.instantiate(&mut store, &module)?;
    Ok((store, linker, instance))
}

pub struct Context<T>
where
    T: events::Events + Default,
{
    pub wasi: WasiCtx,
    pub host: T,
    pub host_tables: events::EventsTables<T>,
}

impl<T> Default for Context<T>
where
    T: events::Events + Default,
{
    fn default() -> Self {
        Self {
            wasi: default_wasi().unwrap(),
            host: Default::default(),
            host_tables: Default::default(),
        }
    }
}
pub type GuestContext = Context<GuestExec>;

fn func_name_to_abi_name(name: &str) -> String {
    name.replace('_', "-")
}
