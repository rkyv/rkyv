#![allow(dead_code, overflowing_literals)]

#[derive(rkyv::Archive)]
#[repr(u8)]
enum Literals {
    Decimal = 1,
    Implicit,
    Binary = 0b0001_0000,
    AfterBinary,
    Octal = 0o40,
    AfterOctal,
    Hexadecimal = 0x40,
    AfterHexadecimal,
    Suffixed = 128u8,
    AfterSuffixed,
    Parenthesized = (200),
    AfterParenthesized,
    Maximum = 255,
}

const _: () = assert!(ArchivedLiterals::Decimal as u8 == 1);
const _: () = assert!(ArchivedLiterals::Implicit as u8 == 2);
const _: () = assert!(ArchivedLiterals::AfterBinary as u8 == 17);
const _: () = assert!(ArchivedLiterals::AfterOctal as u8 == 33);
const _: () = assert!(ArchivedLiterals::AfterHexadecimal as u8 == 65);
const _: () = assert!(ArchivedLiterals::AfterSuffixed as u8 == 129);
const _: () = assert!(ArchivedLiterals::Parenthesized as u8 == 200);
const _: () = assert!(ArchivedLiterals::AfterParenthesized as u8 == 201);
const _: () = assert!(ArchivedLiterals::Maximum as u8 == 255);

// The input enum may have a wider representation. When the standard
// `overflowing_literals` lint is allowed, every generated `u8` tag context
// applies the same truncation to the same literal token.
#[derive(rkyv::Archive)]
#[repr(u16)]
enum AllowedOverflow {
    Zero = 256,
    One,
    Two = 258,
}

const _: () = assert!(ArchivedAllowedOverflow::Zero as u8 == 0);
const _: () = assert!(ArchivedAllowedOverflow::One as u8 == 1);
const _: () = assert!(ArchivedAllowedOverflow::Two as u8 == 2);

// `as = Self` does not generate the public archived enum, but the derive still
// generates its private `u8` tag enum and must preserve the same values there.
#[derive(rkyv::Archive, rkyv::Portable)]
#[rkyv(as = Self)]
#[repr(u8)]
enum AsSelf {
    Literal = 7,
    Implicit,
}

const _: () = assert!(AsSelf::Literal as u8 == 7);
const _: () = assert!(AsSelf::Implicit as u8 == 8);

// Literal metavariables arrive at the derive under an invisible `Expr::Group`.
// The accepted wrapper has no attributes or names and is emitted unchanged.
macro_rules! archive_literal {
    ($value:literal) => {
        #[derive(rkyv::Archive)]
        #[rkyv(derive(Debug))]
        #[repr(u8)]
        enum MacroLiteral {
            Value = $value,
            Implicit,
        }
    };
}

archive_literal!(220);

const _: () = assert!(ArchivedMacroLiteral::Value as u8 == 220);
const _: () = assert!(ArchivedMacroLiteral::Implicit as u8 == 221);

fn main() {}
