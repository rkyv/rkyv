use proc_macro2::{Group, Span, TokenStream, TokenTree};

pub fn respan(stream: TokenStream, span: Span) -> TokenStream {
    stream
        .into_iter()
        .map(|mut token| {
            if let TokenTree::Group(g) = &mut token {
                *g = Group::new(g.delimiter(), respan(g.stream(), span));
            }
            token.set_span(span);
            token
        })
        .collect()
}
