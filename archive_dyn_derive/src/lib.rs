extern crate proc_macro;

// use proc_macro2::{
//     Span,
//     TokenStream,
// };

#[proc_macro_attribute]
pub fn archive_dyn(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    item
}
