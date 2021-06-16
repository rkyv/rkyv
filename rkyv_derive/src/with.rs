use syn::{parse_quote, Expr, Field, Meta, NestedMeta, Path, Type};

#[inline]
pub fn with<B, F: FnMut(B, NestedMeta) -> B>(field: &Field, init: B, f: F) -> B {
    field
        .attrs
        .iter()
        .filter_map(|attr| {
            if let Ok(Meta::List(list)) = attr.parse_meta() {
                if list.path.is_ident("with") {
                    Some(list.nested)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .flatten()
        .rev()
        .fold(init, f)
}

#[inline]
pub fn make_with_ty(rkyv_path: &Path) -> impl '_ + Fn(&Field) -> Type {
    move |field| with(
        field,
        field.ty.clone(),
        |ty, wrapper| parse_quote! { #rkyv_path::with::With<#ty, #wrapper> },
    )
}

#[inline]
pub fn make_with_cast(rkyv_path: &Path) -> impl '_ + Fn(&Field, Expr) -> Expr {
    move |field, expr| with(
        field,
        expr,
        |expr, wrapper| parse_quote! { #rkyv_path::with::With::<_, #wrapper>::cast(#expr) },
    )
}

#[inline]
pub fn with_inner(field: &Field, expr: Expr) -> Expr {
    with(field, expr, |expr, _| parse_quote! { #expr.into_inner() })
}
