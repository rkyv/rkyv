//! Type names for rkyv_dyn.
//!
//! The goal of `TypeName` is to avoid allocations if possible. If all you need is the hash of a
//! type name, then there's no reason to allocate a string to do it.
//!
//! rkyv_typename provides a derive macro to easily implement [`TypeName`], and has options to
//! easily customize your type's name.
//!
//! # Examples
//! ```
//! use rkyv_typename::TypeName;
//! #[derive(TypeName)]
//! #[typename = "CoolType"]
//! struct Example<T>(T);
//!
//! let mut type_name = String::new();
//! Example::<i32>::build_type_name(|piece| type_name += piece);
//! assert_eq!(type_name, "CoolType<i32>");
//! ```
//!
//! ## Features
//!
//! - `std`: Implements [`TypeName`] for standard library types (enabled by default)

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use rkyv_typename_derive::TypeName;
mod typename_impl;
mod const_generic_impl;

/// Builds a name for a type.
///
/// An implementation can be derived automatically with `#[derive(TypeName)]`. See
/// [TypeName](macro@TypeName) for more details.
///
/// Names cannot be guaranteed to be unique and although they are usually suitable to use as keys,
/// precautions should be taken to ensure that if name collisions happen that they are detected and
/// fixable.
///
/// # Examples
///
/// Most of the time, `#[derive(TypeName)]` will suit your needs. However, if you need more control,
/// you can always implement it manually:
///
/// ```
/// use rkyv_typename::TypeName;
///
/// struct Example;
///
/// impl TypeName for Example {
///     fn build_type_name<F: FnMut(&str)>(mut f: F) {
///         f("CoolStruct");
///     }
/// }
///
/// struct GenericExample<T, U, V>(T, U, V);
///
/// impl<
///     T: TypeName,
///     U: TypeName,
///     V: TypeName
/// > TypeName for GenericExample<T, U, V> {
///     fn build_type_name<F: FnMut(&str)>(mut f: F) {
///         f("CoolGeneric<");
///         T::build_type_name(&mut f);
///         f(", ");
///         U::build_type_name(&mut f);
///         f(", ");
///         V::build_type_name(&mut f);
///         f(">");
///     }
/// }
///
/// fn type_name<T: TypeName>() -> String {
///     let mut result = String::new();
///     T::build_type_name(|piece| result += piece);
///     result
/// }
///
/// assert_eq!(type_name::<Example>(), "CoolStruct");
/// assert_eq!(
///     type_name::<GenericExample<i32, Option<String>, Example>>(),
///     "CoolGeneric<i32, core::option::Option<alloc::string::String>, CoolStruct>"
/// );
/// ```
pub trait TypeName {
    /// Submits the pieces of the type name to the given function.
    fn build_type_name<F: FnMut(&str)>(f: F);
}

impl<T: TypeName> TypeName for &T
where
    T: ?Sized,
{
    fn build_type_name<F: FnMut(&str)>(mut f: F) {
        f("&");
        T::build_type_name(f);
    }
}


/// Builds the const generic parameters for a type
///
/// Defining more implementations for this trait is not of much use as long as the compiler only
/// allows a limited set of generic const parameters.
///
/// The interface of this trait may change in the future depending on the future type-system
/// involved with more general contst generics.
#[doc(hidden)]
pub trait ConstGeneric {
    // Submits the const generic parameter to the given function.
    fn build_name<F: FnMut(&str)>(&self, f: F);
}
