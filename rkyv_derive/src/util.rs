use proc_macro2::Ident;

pub fn strip_raw(ident: &Ident) -> String {
    let as_string = ident.to_string();
    as_string
        .strip_prefix("r#")
        .map(ToString::to_string)
        .unwrap_or(as_string)
}
