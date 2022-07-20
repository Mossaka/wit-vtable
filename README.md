# vtable for WIT functions

This repo experiments implementing multiple callback functions in a wasm module that links to WIT bindings. 

## register callback functions
```rust
fn main() -> Result<()> {
    events::Events::new()
        .listen("handle_event1")
        .listen("handle_event2")
        .exec();
    Ok(())
}

#[register_handler]
fn handle_event1(ev: handle_event1::Event) {
    println!("handle_event1 {}", ev.id);
}

#[register_handler]
fn handle_event2(ev: handle_event2::Event) {
    println!("handle_event2 {}", ev.id);
}

```
## run
![run result](/assets/run_result.png)