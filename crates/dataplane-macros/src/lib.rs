use interface::Interface;
use syn::{parse_macro_input, ItemTrait};
mod interface;
use darling::FromMeta;

macro_rules! parse_nested_meta {
    ($ty:ty, $args:expr) => {{
        let meta = match darling::ast::NestedMeta::parse_meta_list(proc_macro2::TokenStream::from(
            $args,
        )) {
            Ok(v) => v,
            Err(e) => {
                return proc_macro::TokenStream::from(darling::Error::from(e).write_errors());
            }
        };

        match <$ty>::from_list(&meta) {
            Ok(object_args) => object_args,
            Err(err) => return proc_macro::TokenStream::from(err.write_errors()),
        }
    }};
}

#[proc_macro_attribute]
pub fn interface(
    args: proc_macro::TokenStream,
    original: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let interface = parse_nested_meta!(Interface, args);
    let item_trait: ItemTrait = parse_macro_input!(original as ItemTrait);
    interface::generate(&interface, &item_trait)
}
