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

## Future proposal: HTTP Router and Server
```rust
fn main() -> Result<()> {
    let router = Router::builder()
        .get("/", "home_handler")
        .get("/users/:userId", "
        ")
        .build()?
    let service = Service::new(router)?;
    Server::bind("localhost:3000").serve(service);
}

#[register_get]
fn home_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let state = req.data::<State>().unwrap();
    println!("State value: {}", state.0);

    Ok(Response::new(Body::from("Home page")))
}

#[register_get]
fn user_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let user_id = req.param("userId").unwrap();
    Ok(Response::new(Body::from(format!("Hello {}", user_id))))
}

```
