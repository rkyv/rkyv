//! # rkyv
//!
//! rkyv (*archive*) is a zero-copy deserialization framework for Rust.
//!
//! It's similar to other zero-copy deserialization frameworks such as
//! [Cap'n Proto](https://capnproto.org) and
//! [FlatBuffers](https://google.github.io/flatbuffers). However, while the
//! former have external schemas and heavily restricted data types, rkyv allows
//! all serialized types to be defined in code and can serialize a wide variety
//! of types that the others cannot. Additionally, rkyv is designed to have
//! little to no overhead, and in most cases will perform exactly the same as
//! native types.
//!
//! rkyv has a hashmap implementation that is built for zero-copy
//! deserialization, so you can serialize your hashmaps with abandon! The
//! implementation is based off of the standard library's `hashbrown` crate and
//! should have nearly identical performance.
//!
//! One of the most impactful features made possible by rkyv is the ability to
//! serialize trait objects and use them *as trait objects* without
//! deserialization. See the `archive_dyn` crate for more details.
//!
//! ## Design
//!
//! Like [serde](https://serde.rs), rkyv uses Rust's powerful trait system to
//! serialize data without the need for reflection. Despite having a wide array
//! of features, you also only pay for what you use. If your data checks out,
//! the serialization process can be as simple as a `memcpy`! Like serde, this
//! allows rkyv to perform at speeds similar to handwritten serializers.
//!
//! Unlike serde, rkyv produces data that is guaranteed deserialization free. If
//! you wrote your data to disk, you can just `mmap` your file into memory, cast
//! a pointer, and your data is ready to use. This makes it ideal for
//! high-performance and IO-limited applications.
//!
//! ## Tradeoffs
//!
//! rkyv is designed primarily for loading bulk game data as efficiently as
//! possible. While rkyv is a great format for final data, it lacks a full
//! schema system and isn't well equipped for data migration. Using a
//! serialization library like serde can help fill these gaps, and you can use
//! serde with the same types as rkyv conflict-free.
//!
//! ## Features
//!
//! - `const_generics`: Improves the implementations for some traits and
//!   provides an [`Archive`] implementation for slices with elements that
//!   implement [`ArchiveSelf`]. Ideal for `#![no_std]` environments.
//! - `inline_more`: Performs more aggressive function inlining.
//! - `more_portable`: Avoids using sse2-optimized intrinsics since they may
//!   cause alignment issues across machines. This feature may go away once any
//!   portability bugs are identified and fixed.
//! - `nightly`: Enables some nightly features, such as
//!   [`likely`](std::intrinsics::likely).
//! - `specialization`: Enables the unfinished specialization feature and
//!   provides more efficient implementations of some functions when working
//!   with [`ArchiveSelf`] types.
//! - `std`: Enables standard library support.
//!
//! By default, the `std` and `inline_more` features are enabled.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    any(feature = "const_generics", feature = "specialization"),
    allow(incomplete_features)
)]
#![cfg_attr(feature = "const_generics", feature(const_generics))]
#![cfg_attr(feature = "nightly", feature(core_intrinsics))]
#![cfg_attr(feature = "specialization", feature(specialization))]

pub mod core_impl;
#[cfg(feature = "std")]
pub mod hashmap_impl;
#[cfg(feature = "std")]
pub mod std_impl;
#[cfg(feature = "validation")]
pub mod validation;

use core::{
    marker::PhantomPinned,
    mem,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr, slice,
};
#[cfg(feature = "std")]
use std::io;

pub use memoffset::offset_of;
pub use rkyv_derive::Archive;
#[cfg(feature = "validation")]
pub use validation::{check_archive, ArchiveContext};

/// A `#![no_std]` compliant writer that knows where it is.
///
/// A type that is [`io::Write`](std::io::Write) can be wrapped in an
/// ArchiveWriter to equip it with `Write`. It's important that the memory for
/// archived objects is properly aligned before attempting to read objects out
/// of it, use the [`Aligned`] wrapper if it's appropriate.
pub trait Write {
    /// The errors that may occur while writing.
    type Error: 'static;

    /// Returns the current position of the writer.
    fn pos(&self) -> usize;

    /// Attempts to write the given bytes to the writer.
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

/// Helper functions on [`Write`] objects.
pub trait WriteExt: Write {
    /// Aligns the position of the writer to the given alignment.
    fn align(&mut self, align: usize) -> Result<usize, Self::Error> {
        debug_assert!(align & (align - 1) == 0);

        let offset = self.pos() & (align - 1);
        if offset != 0 {
            const ZEROES_LEN: usize = 16;
            const ZEROES: [u8; ZEROES_LEN] = [0; ZEROES_LEN];

            let mut padding = align - offset;
            loop {
                let len = usize::min(ZEROES_LEN, padding);
                self.write(&ZEROES[0..len])?;
                padding -= len;
                if padding == 0 {
                    break;
                }
            }
        }
        Ok(self.pos())
    }

    /// Aligns the position of the writer to be suitable to write the given
    /// type.
    fn align_for<T>(&mut self) -> Result<usize, Self::Error> {
        self.align(mem::align_of::<T>())
    }

    /// Resolves the given resolver and writes its archived type, returning the
    /// position of the written archived type.
    ///
    /// # Safety
    ///
    /// This is only safe to call when the writer is already aligned for the
    /// archived version of the given type.
    unsafe fn resolve_aligned<T: ?Sized, R: Resolve<T>>(
        &mut self,
        value: &T,
        resolver: R,
    ) -> Result<usize, Self::Error> {
        let pos = self.pos();
        debug_assert!(pos & (mem::align_of::<R::Archived>() - 1) == 0);
        let archived = &resolver.resolve(pos, value);
        let data = (archived as *const R::Archived).cast::<u8>();
        let len = mem::size_of::<R::Archived>();
        self.write(slice::from_raw_parts(data, len))?;
        Ok(pos)
    }

    /// Archives the given object and returns the position it was archived at.
    fn archive<T: Archive>(&mut self, value: &T) -> Result<usize, Self::Error> {
        let resolver = value.archive(self)?;
        self.align_for::<T::Archived>()?;
        unsafe { self.resolve_aligned(value, resolver) }
    }

    /// Archives a reference to the given object and returns the position it was
    /// archived at.
    fn archive_ref<T: ArchiveRef + ?Sized>(&mut self, value: &T) -> Result<usize, Self::Error> {
        let resolver = value.archive_ref(self)?;
        self.align_for::<T::Reference>()?;
        unsafe { self.resolve_aligned(value, resolver) }
    }
}

impl<W: Write + ?Sized> WriteExt for W {}

/// Creates an archived value when given a value and position.
///
/// Resolvers are passed the original value, so any information that is already
/// in them doesn't have to be stored in the resolver.
pub trait Resolve<T: ?Sized> {
    /// The type that this resolver resolves to.
    type Archived;

    /// Creates the archived version of the given value at the given position.
    fn resolve(self, pos: usize, value: &T) -> Self::Archived;
}

/// Writes a type to a [`Writer`](Write) so it can be used without
/// deserializing.
///
/// Archiving is done depth-first, writing any data owned by a type before
/// writing the data for the type itself. The [`Resolver`](Resolve) must be able
/// to create the archived type from only its own data and the value being
/// archived.
///
/// ## Examples
///
/// Most of the time, `#[derive(Archive)]` will create an acceptable
/// implementation. You can use the `#[archive(...)]` attribute to control how
/// the implementation is generated. See the [`Archive`](macro@Archive) derive
/// macro for more details.
///
/// ```
/// use rkyv::{Aligned, Archive, ArchiveBuffer, Archived, archived_value, WriteExt};
///
/// #[derive(Archive)]
/// struct Test {
///     int: u8,
///     string: String,
///     option: Option<Vec<i32>>,
/// }
///
/// let mut writer = ArchiveBuffer::new(Aligned([0u8; 256]));
/// let value = Test {
///     int: 42,
///     string: "hello world".to_string(),
///     option: Some(vec![1, 2, 3, 4]),
/// };
/// let pos = writer.archive(&value)
///     .expect("failed to archive test");
/// let buf = writer.into_inner();
/// let archived = unsafe { archived_value::<Test, _>(&buf, pos) };
/// assert_eq!(archived.int, value.int);
/// assert_eq!(archived.string, value.string);
/// assert_eq!(archived.option, value.option);
/// ```
///
/// Many of the core and standard library types already have Archive
/// implementations available, but you may need to implement `Archive` for your
/// own types in some cases the derive macro cannot handle.
///
/// In this example, we add our own wrapper that serializes a `&'static str` as
/// if it's owned. Normally you can lean on the archived version of `String` to
/// do most of the work, but this example does everything to demonstrate how to
/// implement `Archive` for your own types.
///
/// ```
/// use core::{slice, str};
/// use rkyv::{
///     Aligned,
///     Archive,
///     ArchiveBuffer,
///     Archived,
///     archived_value,
///     offset_of,
///     RelPtr,
///     Resolve,
///     Write,
///     WriteExt,
/// };
///
/// struct OwnedStr {
///     inner: &'static str,
/// }
///
/// struct ArchivedOwnedStr {
///     // This will be a relative pointer to the bytes of our string.
///     ptr: RelPtr,
///     // The length of the archived version must be explicitly sized for
///     // 32/64-bit compatibility. Archive is not implemented for usize and
///     // isize to help you avoid making this mistake.
///     len: u32,
/// }
///
/// impl ArchivedOwnedStr {
///     // This will help us get the bytes of our type as a str again.
///     fn as_str(&self) -> &str {
///         unsafe {
///             // The as_ptr() function of RelPtr will get a pointer
///             // to its memory.
///             let bytes = slice::from_raw_parts(self.ptr.as_ptr(), self.len as usize);
///             str::from_utf8_unchecked(bytes)
///         }
///     }
/// }
///
/// struct OwnedStrResolver {
///     // This will be the position that the bytes of our string are stored at.
///     // We'll use this to make the relative pointer of our ArchivedOwnedStr.
///     bytes_pos: usize,
/// }
///
/// impl Resolve<OwnedStr> for OwnedStrResolver {
///     // This is essentially the output type of the resolver. It must match
///     // the Archived associated type in our impl of Archive for OwnedStr.
///     type Archived = ArchivedOwnedStr;
///
///     // The resolve function consumes the resolver and produces the archived
///     // value at the given position.
///     fn resolve(self, pos: usize, value: &OwnedStr) -> Self::Archived {
///         Self::Archived {
///             // We have to be careful to add the offset of the ptr field,
///             // otherwise we'll be using the position of the ArchivedOwnedStr
///             // instead of the position of the ptr. That's the reason why
///             // RelPtr::new is unsafe.
///             ptr: unsafe {
///                 RelPtr::new(pos + offset_of!(ArchivedOwnedStr, ptr), self.bytes_pos)
///             },
///             len: value.inner.len() as u32,
///         }
///     }
/// }
///
/// impl Archive for OwnedStr {
///     type Archived = ArchivedOwnedStr;
///     /// This is the resolver we'll return from archive.
///     type Resolver = OwnedStrResolver;
///
///     fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
///         // This is where we want to write the bytes of our string and return
///         // a resolver that knows where those bytes were written.
///         let bytes_pos = writer.pos();
///         writer.write(self.inner.as_bytes())?;
///         Ok(Self::Resolver { bytes_pos })
///     }
/// }
///
/// let mut writer = ArchiveBuffer::new(Aligned([0u8; 256]));
/// const STR_VAL: &'static str = "I'm in an OwnedStr!";
/// let value = OwnedStr { inner: STR_VAL };
/// // It works!
/// let pos = writer.archive(&value)
///     .expect("failed to archive test");
/// let buf = writer.into_inner();
/// let archived = unsafe { archived_value::<OwnedStr, _>(&buf, pos) };
/// // Let's make sure our data got written correctly
/// assert_eq!(archived.as_str(), STR_VAL);
/// ```
pub trait Archive {
    /// The archived version of this type.
    type Archived;
    /// The resolver for this type. It must contain all the information needed
    /// to make the archived type from the unarchived type.
    type Resolver: Resolve<Self, Archived = Self::Archived>;

    /// Writes the dependencies for the object and returns a resolver that can
    /// create the archived type.
    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error>;
}

/// This trait is a counterpart of [`Archive`] that's suitable for unsized
/// types.
///
/// Instead of archiving its value directly, `ArchiveRef` archives a type that
/// dereferences to its archived type. As a consequence, its `Resolver` resolves
/// to a `Reference` instead of the archived type.
///
/// `ArchiveRef` is automatically implemented for all types that implement
/// [`Archive`], and uses a [`RelPtr`] as the reference type.
///
/// `ArchiveRef` is already implemented for slices and string slices. Use the
/// `rkyv_dyn` crate to archive trait objects. Unfortunately, you'll have to
/// manually implement `ArchiveRef` for your other unsized types.
pub trait ArchiveRef {
    /// The archived version of this type.
    type Archived: ?Sized;
    /// The reference to the archived version of this type.
    type Reference: Deref<Target = Self::Archived> + DerefMut<Target = Self::Archived>;
    /// The resolver for the reference of this type.
    type Resolver: Resolve<Self, Archived = Self::Reference>;

    /// Writes the object and returns a resolver that can create the reference
    /// to the archived type.
    fn archive_ref<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error>;
}

/// A trait that indicates that some [`Archive`] type can be copied directly to
/// an archive without additional processing.
///
/// You can derive an implementation of `ArchiveSelf` by adding
/// `#[archive(self)]` to the struct or enum. Types that implement `ArchiveSelf`
/// must also implement [`Copy`](core::marker::Copy).
///
/// Types that implement `ArchiveSelf` are not guaranteed to have `archive`
/// called on them to archive their value. Most or all implementations that
/// leverage `ArchiveSelf` will require the `specialization` feature.
///
/// `ArchiveSelf` must be manually implemented even if a type implements
/// [`Archive`] and [`Copy`](core::marker::Copy) because some types may
/// transform their data when writing to an archive.
///
/// ## Examples
/// ```
/// use rkyv::{Aligned, Archive, ArchiveBuffer, archived_value, Write, WriteExt};
///
/// #[derive(Archive, Clone, Copy, Debug, PartialEq)]
/// #[archive(self)]
/// struct Vector4<T>(T, T, T, T);
///
/// let mut writer = ArchiveBuffer::new(Aligned([0u8; 256]));
/// let value = Vector4(1f32, 2f32, 3f32, 4f32);
/// let pos = writer.archive(&value)
///     .expect("failed to archive Vector4");
/// let buf = writer.into_inner();
/// let archived_value = unsafe { archived_value::<Vector4<f32>, _>(&buf, pos) };
/// assert_eq!(&value, archived_value);
/// ```
pub unsafe trait ArchiveSelf: Archive<Archived = Self> + Copy {}

/// A resolver that always resolves to the unarchived value. This can be useful
/// while implementing [`ArchiveSelf`].
pub struct SelfResolver;

impl<T: ArchiveSelf> Resolve<T> for SelfResolver {
    type Archived = T;

    fn resolve(self, _pos: usize, value: &T) -> T {
        *value
    }
}

/// A pointer which resolves to relative to its position in memory.
///
/// See [`Archive`] for an example of creating one.
#[repr(transparent)]
#[derive(Debug)]
pub struct RelPtr {
    offset: i32,
    _phantom: PhantomPinned,
}

impl RelPtr {
    /// Creates a relative pointer from one position to another.
    ///
    /// # Safety
    ///
    /// `from` must be the position of the relative pointer and `to` must be the
    /// position of some valid memory.
    pub unsafe fn new(from: usize, to: usize) -> Self {
        Self {
            offset: (to as isize - from as isize) as i32,
            _phantom: PhantomPinned,
        }
    }

    /// Calculates the memory address being pointed to by this relative pointer.
    pub fn as_ptr<T>(&self) -> *const T {
        unsafe {
            (self as *const Self)
                .cast::<u8>()
                .offset(self.offset as isize)
                .cast::<T>()
        }
    }

    /// Returns an unsafe mutable pointer to the memory address being pointed to
    /// by this relative pointer.
    pub fn as_mut_ptr<T>(&mut self) -> *mut T {
        unsafe {
            (self as *mut Self)
                .cast::<u8>()
                .offset(self.offset as isize)
                .cast::<T>()
        }
    }
}

/// Alias for the archived version of some [`Archive`] type.
pub type Archived<T> = <T as Archive>::Archived;
/// Alias for the resolver for some [`Archive`] type.
pub type Resolver<T> = <T as Archive>::Resolver;
/// Alias for the resolver of the reference for some [`ArchiveRef`] type.
pub type ReferenceResolver<T> = <T as ArchiveRef>::Resolver;
/// Alias for the reference for some [`ArchiveRef`] type.
pub type Reference<T> = <T as ArchiveRef>::Reference;

/// Wraps a type and aligns it to at least 16 bytes. Mainly used to align byte
/// buffers for [ArchiveBuffer].
///
/// ## Examples
/// ```
/// use core::mem;
/// use rkyv::Aligned;
///
/// assert_eq!(mem::align_of::<u8>(), 1);
/// assert_eq!(mem::align_of::<Aligned<u8>>(), 16);
/// ```
#[repr(align(16))]
pub struct Aligned<T>(pub T);

impl<T: Deref> Deref for Aligned<T> {
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T: DerefMut> DerefMut for Aligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl<T: AsRef<[U]>, U> AsRef<[U]> for Aligned<T> {
    fn as_ref(&self) -> &[U] {
        self.0.as_ref()
    }
}

impl<T: AsMut<[U]>, U> AsMut<[U]> for Aligned<T> {
    fn as_mut(&mut self) -> &mut [U] {
        self.0.as_mut()
    }
}

/// Wraps a byte buffer and writes into it.
///
/// Common uses include archiving in `#![no_std]` environments and archiving
/// small objects without allocating.
///
/// ## Examples
/// ```
/// use rkyv::{Aligned, Archive, ArchiveBuffer, Archived, archived_value, WriteExt};
///
/// #[derive(Archive)]
/// enum Event {
///     Spawn,
///     Speak(String),
///     Die,
/// }
///
/// let mut writer = ArchiveBuffer::new(Aligned([0u8; 256]));
/// let pos = writer.archive(&Event::Speak("Help me!".to_string()))
///     .expect("failed to archive event");
/// let buf = writer.into_inner();
/// let archived = unsafe { archived_value::<Event, _>(&buf, pos) };
/// if let Archived::<Event>::Speak(message) = archived {
///     assert_eq!(message.as_str(), "Help me!");
/// } else {
///     panic!("archived event was of the wrong type");
/// }
/// ```
pub struct ArchiveBuffer<T> {
    inner: T,
    pos: usize,
}

impl<T> ArchiveBuffer<T> {
    /// Creates a new archive buffer from a byte buffer.
    pub fn new(inner: T) -> Self {
        Self::with_pos(inner, 0)
    }

    /// Creates a new archive buffer from a byte buffer. The buffer will start
    /// writing at the given position, but the buffer must contain all bytes
    /// (otherwise the alignments of types may not be correct).
    pub fn with_pos(inner: T, pos: usize) -> Self {
        Self { inner, pos }
    }

    /// Consumes the buffer and returns the internal buffer used to create it.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

/// The error type returned by an [`ArchiveBuffer`].
#[derive(Debug)]
pub enum ArchiveBufferError {
    /// Writing has overflowed the internal buffer.
    Overflow,
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Write for ArchiveBuffer<T> {
    type Error = ArchiveBufferError;

    fn pos(&self) -> usize {
        self.pos
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        let end_pos = self.pos + bytes.len();
        if end_pos > self.inner.as_ref().len() {
            Err(ArchiveBufferError::Overflow)
        } else {
            unsafe {
                ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    self.inner.as_mut().as_mut_ptr().add(self.pos),
                    bytes.len(),
                );
            }
            self.pos = end_pos;
            Ok(())
        }
    }
}

/// Wraps a type that implements [`io::Write`](std::io::Write) and equips it
/// with [`Write`].
///
/// ## Examples
/// ```
/// use rkyv::{ArchiveWriter, Write};
///
/// let mut writer = ArchiveWriter::new(Vec::new());
/// assert_eq!(writer.pos(), 0);
/// writer.write(&[0u8, 1u8, 2u8, 3u8]);
/// assert_eq!(writer.pos(), 4);
/// let buf = writer.into_inner();
/// assert_eq!(buf.len(), 4);
/// assert_eq!(buf, vec![0u8, 1u8, 2u8, 3u8]);
/// ```
#[cfg(feature = "std")]
pub struct ArchiveWriter<W: io::Write> {
    inner: W,
    pos: usize,
}

#[cfg(feature = "std")]
impl<W: io::Write> ArchiveWriter<W> {
    /// Creates a new archive writer from a writer.
    pub fn new(inner: W) -> Self {
        Self::with_pos(inner, 0)
    }

    /// Creates a new archive writer from a writer, and assumes that the
    /// underlying writer is currently at the given position.
    pub fn with_pos(inner: W, pos: usize) -> Self {
        Self { inner, pos }
    }

    /// Consumes the writer and returns the internal writer used to create it.
    pub fn into_inner(self) -> W {
        self.inner
    }
}

#[cfg(feature = "std")]
impl<W: io::Write> Write for ArchiveWriter<W> {
    type Error = io::Error;

    fn pos(&self) -> usize {
        self.pos
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.pos += self.inner.write(bytes)?;
        Ok(())
    }
}

/// Casts an archived value from the given byte array at the given position.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and
/// allow buffer mutation after getting archived value references.
///
/// # Safety
///
/// This is only safe to call if the value is archived at the given position in
/// the byte array.
#[inline]
pub unsafe fn archived_value<T: Archive + ?Sized, U: AsRef<[u8]> + ?Sized>(
    bytes: &U,
    pos: usize,
) -> &Archived<T> {
    &*bytes.as_ref().as_ptr().add(pos).cast()
}

/// Casts a mutable archived value from the given byte array at the given
/// position.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and
/// allow buffer mutation after getting archived value references.
///
/// # Safety
///
/// This is only safe to call if the value is archived at the given position in
/// the byte array.
#[inline]
pub unsafe fn archived_value_mut<T: Archive + ?Sized, U: AsMut<[u8]> + ?Sized>(
    bytes: Pin<&mut U>,
    pos: usize,
) -> Pin<&mut Archived<T>> {
    Pin::new_unchecked(
        &mut *bytes
            .get_unchecked_mut()
            .as_mut()
            .as_mut_ptr()
            .add(pos)
            .cast(),
    )
}

/// Casts an archived reference from the given byte array at the given position.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and
/// allow buffer mutation after getting archived value references.
///
/// # Safety
///
/// This is only safe to call if the reference is archived at the given position
/// in the byte array.
#[inline]
pub unsafe fn archived_ref<T: ArchiveRef + ?Sized, U: AsRef<[u8]> + ?Sized>(
    bytes: &U,
    pos: usize,
) -> &Reference<T> {
    &*bytes.as_ref().as_ptr().add(pos).cast()
}

/// Casts a mutable archived reference from the given byte array at the given
/// position.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and
/// allow buffer mutation after getting archived value references.
///
/// # Safety
///
/// This is only safe to call if the reference is archived at the given position
/// in the byte array.
#[inline]
pub unsafe fn archived_ref_mut<T: ArchiveRef + ?Sized, U: AsMut<[u8]> + ?Sized>(
    bytes: Pin<&mut U>,
    pos: usize,
) -> Pin<&mut Reference<T>> {
    Pin::new_unchecked(
        &mut *bytes
            .get_unchecked_mut()
            .as_mut()
            .as_mut_ptr()
            .add(pos)
            .cast(),
    )
}
