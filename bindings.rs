pub mod handler {
  #[allow(unused_imports)]
  use wit_bindgen_wasmtime::{wasmtime, anyhow};
  #[derive(Clone)]
  pub struct Event<'a,> {
    pub id: &'a  str,
    pub data: &'a  str,
  }
  impl<'a,> std::fmt::Debug for Event<'a,> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.debug_struct("Event").field("id", &self.id).field("data", &self.data).finish()}
  }
  
  /// Auxiliary data associated with the wasm exports.
  ///
  /// This is required to be stored within the data of a
  /// `Store<T>` itself so lifting/lowering state can be managed
  /// when translating between the host and wasm.
  #[derive(Default)]
  pub struct HandlerData {
  }
  pub struct Handler<T> {
    get_state: Box<dyn Fn(&mut T) -> &mut HandlerData + Send + Sync>,
    canonical_abi_realloc: wasmtime::TypedFunc<(i32, i32, i32, i32), i32>,
    handler: wasmtime::TypedFunc<(i32,i32,i32,i32,), ()>,
    memory: wasmtime::Memory,
  }
  impl<T> Handler<T> {
    #[allow(unused_variables)]
    
    /// Adds any intrinsics, if necessary for this exported wasm
    /// functionality to the `linker` provided.
    ///
    /// The `get_state` closure is required to access the
    /// auxiliary data necessary for these wasm exports from
    /// the general store's state.
    pub fn add_to_linker(
    linker: &mut wasmtime::Linker<T>,
    get_state: impl Fn(&mut T) -> &mut HandlerData + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
      Ok(())
    }
    
    /// Instantiates the provided `module` using the specified
    /// parameters, wrapping up the result in a structure that
    /// translates between wasm and the host.
    ///
    /// The `linker` provided will have intrinsics added to it
    /// automatically, so it's not necessary to call
    /// `add_to_linker` beforehand. This function will
    /// instantiate the `module` otherwise using `linker`, and
    /// both an instance of this structure and the underlying
    /// `wasmtime::Instance` will be returned.
    ///
    /// The `get_state` parameter is used to access the
    /// auxiliary state necessary for these wasm exports from
    /// the general store state `T`.
    pub fn instantiate(
    mut store: impl wasmtime::AsContextMut<Data = T>,
    module: &wasmtime::Module,
    linker: &mut wasmtime::Linker<T>,
    get_state: impl Fn(&mut T) -> &mut HandlerData + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<(Self, wasmtime::Instance)> {
      Self::add_to_linker(linker, get_state)?;
      let instance = linker.instantiate(&mut store, module)?;
      Ok((Self::new(store, &instance,get_state)?, instance))
    }
    
    /// Low-level creation wrapper for wrapping up the exports
    /// of the `instance` provided in this structure of wasm
    /// exports.
    ///
    /// This function will extract exports from the `instance`
    /// defined within `store` and wrap them all up in the
    /// returned structure which can be used to interact with
    /// the wasm module.
    pub fn new(
    mut store: impl wasmtime::AsContextMut<Data = T>,
    instance: &wasmtime::Instance,
    get_state: impl Fn(&mut T) -> &mut HandlerData + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<Self> {
      let mut store = store.as_context_mut();
      let canonical_abi_realloc= instance.get_typed_func::<(i32, i32, i32, i32), i32, _>(&mut store, "canonical_abi_realloc")?;
      let handler= instance.get_typed_func::<(i32,i32,i32,i32,), (), _>(&mut store, "handler")?;
      let memory= instance
      .get_memory(&mut store, "memory")
      .ok_or_else(|| {
        anyhow::anyhow!("`memory` export not a memory")
      })?
      ;
      Ok(Handler{
        canonical_abi_realloc,
        handler,
        memory,
        get_state: Box::new(get_state),
        
      })
    }
    pub fn handler(&self, mut caller: impl wasmtime::AsContextMut<Data = T>,ev: Event<'_,>,)-> Result<(), wasmtime::Trap> {
      let func_canonical_abi_realloc = &self.canonical_abi_realloc;
      let memory = &self.memory;
      let Event{ id:id0, data:data0, } = ev;
      let vec1 = id0;
      let ptr1 = func_canonical_abi_realloc.call(&mut caller, (0, 0, 1, vec1.len() as i32))?;
      memory.data_mut(&mut caller).store_many(ptr1, vec1.as_bytes())?;
      let vec2 = data0;
      let ptr2 = func_canonical_abi_realloc.call(&mut caller, (0, 0, 1, vec2.len() as i32))?;
      memory.data_mut(&mut caller).store_many(ptr2, vec2.as_bytes())?;
      self.handler.call(&mut caller, (ptr1, vec1.len() as i32, ptr2, vec2.len() as i32, ))?;
      Ok(())
    }
  }
  use wit_bindgen_wasmtime::rt::RawMem;
}
