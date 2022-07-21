use anyhow::Result;

use sdk::register_handler;
wit_bindgen_rust::import!("../events.wit");

fn main() -> Result<()> {
    println!("hello from wasm!");
    events::Events::new()
        .listen("handle_event1")
        .listen("handle_event2")
        .listen("handle_event3")
        .exec(5);
    println!("finished!");
    Ok(())
}

/// The guest has to register a dummy function, in which the name matches
/// the name of the function defined in the WIT file. This is because the
/// host will, by default, instantiate a struct that tries to get this function
/// from the WIT file.
#[register_handler]
fn handler(_ev: handler::Event) {
    // dummy, not used
    
}

#[register_handler]
fn handle_event1(ev: handle_event1::Event) {
    println!("handle_event1 {}", ev.id);
}

#[register_handler]
fn handle_event2(ev: handle_event2::Event) {
    println!("handle_event2 {}", ev.id);
}

#[register_handler]
fn handle_event3(ev: handle_event3::Event) {
    println!("handle_event3 {}", ev.id);
}
