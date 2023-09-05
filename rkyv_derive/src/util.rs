use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    token::Token as TokenTrait,
    Error, LitStr, Token, WhereClause,
};

pub fn add_bounds(
    bounds: &LitStr,
    where_clause: &mut WhereClause,
) -> Result<(), Error> {
    let clauses = bounds.parse_with(Vec::parse_terminated::<Token![,]>)?;

    for clause in clauses {
        where_clause.predicates.push(clause);
    }

    Ok(())
}

pub fn strip_raw(ident: &Ident) -> String {
    let as_string = ident.to_string();
    as_string
        .strip_prefix("r#")
        .map(ToString::to_string)
        .unwrap_or(as_string)
}

/// Revamping utility from [`Punctuated`] for the purpose of storing items
/// more efficiently.
///
/// [`Punctuated`]: syn::punctuated::Punctuated
pub trait PunctuatedExt<T> {
    /// Parses one or more occurrences of `T` separated by punctuation of type
    /// `P`, not accepting trailing punctuation.
    ///
    /// Parsing continues as long as punctuation `P` is present at the head of
    /// the stream. This method returns upon parsing a `T` and observing that it
    /// is not followed by a `P`, even if there are remaining tokens in the
    /// stream.
    fn parse_separated_nonempty<P: Parse + TokenTrait>(
        input: ParseStream,
    ) -> Result<Vec<T>, Error>
    where
        T: Parse,
    {
        Self::parse_separated_nonempty_with::<P>(input, T::parse)
    }

    /// Parses one or more occurrences of `T` using the given parse function,
    /// separated by punctuation of type `P`, not accepting trailing
    /// punctuation.
    ///
    /// Like [`parse_separated_nonempty`], may complete early without parsing
    /// the entire content of this stream.
    fn parse_separated_nonempty_with<P: Parse + TokenTrait>(
        input: ParseStream,
        parser: fn(ParseStream) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error>;

    /// Parses zero or more occurrences of `T` separated by punctuation of type
    /// `P`, with optional trailing punctuation.
    ///
    /// Parsing continues until the end of this parse stream. The entire content
    /// of this parse stream must consist of `T` and `P`.
    fn parse_terminated<P: Parse>(input: ParseStream) -> Result<Vec<T>, Error>
    where
        T: Parse,
    {
        Self::parse_terminated_with::<P>(input, T::parse)
    }

    /// Parses zero or more occurrences of `T` using the given parse function,
    /// separated by punctuation of type `P`, with optional trailing
    /// punctuation.
    ///
    /// Like [`parse_terminated`], the entire content of this stream is expected
    /// to be parsed.
    fn parse_terminated_with<P: Parse>(
        input: ParseStream,
        parser: fn(ParseStream) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error>;
}

impl<T> PunctuatedExt<T> for Vec<T> {
    fn parse_separated_nonempty_with<P: Parse + TokenTrait>(
        input: ParseStream,
        parser: fn(ParseStream) -> Result<T, Error>,
    ) -> Result<Self, Error> {
        let mut vec = Vec::new();

        loop {
            vec.push(parser(input)?);

            if !P::peek(input.cursor()) {
                break;
            }

            input.parse::<P>()?;
        }

        Ok(vec)
    }

    fn parse_terminated_with<P: Parse>(
        input: ParseStream,
        parser: fn(ParseStream) -> Result<T, Error>,
    ) -> Result<Self, Error> {
        let mut vec = Vec::new();

        loop {
            if input.is_empty() {
                break;
            }

            vec.push(parser(input)?);

            if input.is_empty() {
                break;
            }

            input.parse::<P>()?;
        }

        Ok(vec)
    }
}
