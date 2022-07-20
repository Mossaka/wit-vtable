use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Result;
// use event_handler::{EventHandler, EventHandlerData};
use events::EventsTables;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{StringArrayError, WasiCtx};
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::*;
use wit_bindgen_wasmtime::rt::RawMem;

// wit_bindgen_wasmtime::import!("event-handler.wit");
wit_bindgen_wasmtime::export!("events.wit");

// FIXME: maybe Event could be generated from the bindings file?
#[derive(Clone)]
pub struct Event<'a> {
    pub id: &'a str,
    pub data: &'a str,
}
impl<'a> std::fmt::Debug for Event<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event")
            .field("id", &self.id)
            .field("data", &self.data)
            .finish()
    }
}

#[derive(Default)]
pub struct Exec {
    guest_instance: Option<Arc<Mutex<Instance>>>,
    guest_store: Option<Arc<Mutex<Store<GuestContext>>>>,
    vtable: Vec<String>,
}

impl events::Events for Exec {
    type Events = ();

    fn events_new(&mut self) -> Self::Events {
        ()
    }

    fn events_listen(&mut self, _self_: &Self::Events, id: &str) -> Self::Events {
        self.vtable.push(id.to_string());
        ()
    }

    fn events_exec(&mut self, _self_: &Self::Events, _duration: u64) -> Self::Events {
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
                    call_handle(
                        &func_name_to_abi_name(func_name),
                        store.deref_mut(),
                        instance.deref(),
                        ev,
                    );
                }
            }));
        }
        for handle in thread_handles {
            handle.join().unwrap();
        }
        ()
    }
}

fn call_handle(
    name: &str,
    mut store: impl wasmtime::AsContextMut<Data = GuestContext>,
    instance: &wasmtime::Instance,
    ev: Event<'_>,
) -> Result<()> {
    let mut store = store.as_context_mut();
    let canonical_abi_realloc = instance
        .get_typed_func::<(i32, i32, i32, i32), i32, _>(&mut store, "canonical_abi_realloc")?;
    let handler = instance.get_typed_func::<(i32, i32, i32, i32), (), _>(&mut store, name)?;
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow::anyhow!("`memory` export not a memory"))?;
    let Event {
        id: id0,
        data: data0,
    } = ev;
    let vec1 = id0;
    let ptr1 = canonical_abi_realloc.call(&mut store, (0, 0, 1, vec1.len() as i32))?;
    memory
        .data_mut(&mut store)
        .store_many(ptr1, vec1.as_bytes())?;
    let vec2 = data0;
    let ptr2 = canonical_abi_realloc.call(&mut store, (0, 0, 1, vec2.len() as i32))?;
    memory
        .data_mut(&mut store)
        .store_many(ptr2, vec2.as_bytes())?;
    handler.call(
        &mut store,
        (ptr1, vec1.len() as i32, ptr2, vec2.len() as i32),
    )?;
    Ok(())
}

#[derive(Default)]
pub struct GuestExec {
    // data: EventHandlerData,
}

impl events::Events for GuestExec {
    type Events = ();

    fn events_new(&mut self) -> Self::Events {
        ()
    }

    fn events_listen(&mut self, _self_: &Self::Events, id: &str) -> Self::Events {
        ()
    }

    fn events_exec(&mut self, _self_: &Self::Events, _duration: u64) -> Self::Events {
        ()
    }
}

fn main() -> Result<()> {
    let engine = Engine::new(&default_config()?)?;
    let path = "target/wasm32-wasi/debug/guest.wasm";

    let (mut store, _linker, instance) = wasmtime_init(&engine, path)?;
    let (mut store2, _linker2, instance2) = wasmtime_init(&engine, path)?;

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
    let n = name.replace("_", "-");
    format!("handle-{}", n)
}
