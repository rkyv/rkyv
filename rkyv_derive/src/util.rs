use proc_macro2::Ident;
use syn::{Field, Meta};

pub fn strip_raw(ident: &Ident) -> String {
    let as_string = ident.to_string();
    as_string
        .strip_prefix("r#")
        .map(ToString::to_string)
        .unwrap_or(as_string)
}

pub fn is_not_omitted(f: &&Field) -> bool {
    f.attrs.iter().all(|attr| {
        if let Meta::Path(path) = &attr.meta {
            !path.is_ident("omit_bounds")
        } else {
            true
        }
    })
}
