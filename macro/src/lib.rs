use proc_macro::TokenStream;
use quote::quote;

use wit_bindgen_gen_core::{wit_parser::Interface, Direction, Files, Generator};
use wit_bindgen_gen_rust_wasm::RustWasm;

fn some_kind_of_uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect(),
    }
}

fn underscore_to_hyphen(s: &str) -> String {
    s.replace('_', "-")
}

/// Register handler
#[proc_macro_attribute]
pub fn register_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;
    let handle_func = format!("{}", func_name);
    let handle_func_wit = underscore_to_hyphen(&handle_func);
    let event = r#"
record event {
    id: string,
    data: string,
}

        "#
    .to_string();
    let handle_func_wit =
        event + format!("{}: function(ev: event) -> unit\n        ", handle_func_wit).as_str();
    let iface = Interface::parse(&func_name.to_string(), &handle_func_wit).expect("parse error");
    let handle_func = syn::parse_str::<syn::Ident>(&handle_func).expect("parse error");
    let mut files = Files::default();
    let mut rust_wasm = RustWasm::new();
    rust_wasm.generate_one(&iface, Direction::Export, &mut files);
    let (_, contents) = files.iter().next().unwrap();
    let iface_tokens: TokenStream = std::str::from_utf8(contents)
        .expect("cannot parse UTF-8 from interface file")
        .parse()
        .expect("cannot parse interface file");
    let iface = syn::parse_macro_input!(iface_tokens as syn::ItemMod);
    let struct_name = func_name
        .to_string()
        .split('_')
        .into_iter()
        .map(some_kind_of_uppercase_first_letter)
        .collect::<String>();
    let struct_ident = syn::parse_str::<syn::Ident>(&struct_name).unwrap();
    quote!(
        #iface

        struct #struct_ident {}
        impl #func_name::#struct_ident for #struct_ident {
            fn #handle_func(ev: #func_name::Event) {
                #func

                #func_name(ev)
            }
        }
    )
    .into()
}
