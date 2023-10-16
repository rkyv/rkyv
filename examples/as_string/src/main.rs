use rkyv::{
    archived_root,
    ser::{serializers::AllocSerializer, Serializer},
    with::{AsString, AsStringError},
    Archive, Fallible, Serialize,
};
use std::path::{Path, PathBuf};

// This is the struct we'll be serializing. It uses the AsString wrapper, which requires a
// serializer that satisfies <S as Fallible>::Error: From<AsStringError>. In order to satisfy that,
// we need to make a new serializer and wrap it.
#[derive(Archive, Serialize)]
pub struct Example {
    #[with(AsString)]
    path: PathBuf,
}

// This will be our serializer wrappper, it just contains another serializer inside of it and
// forwards everything down.
struct MySerializer<S> {
    inner: S,
}

impl<S> MySerializer<S> {
    pub fn into_inner(self) -> S {
        self.inner
    }
}

// The Fallible trait defines the error type for our serializer. This is our new error type that
// will implement From<AsStringError>.
impl<S: Fallible> Fallible for MySerializer<S> {
    type Error = MySerializerError<E>;
}

// Our Serializer impl just forwards everything down to the inner serializer.
impl<S: Serializer> Serializer for MySerializer<S> {
    #[inline]
    fn pos(&self) -> usize {
        self.inner.pos()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.write(bytes).map_err(MySerializerError::Inner)
    }
}

// A Default implementation will make it easier to construct our serializer in some cases.
impl<S: Default> Default for MySerializer<S> {
    fn default() -> Self {
        Self {
            inner: S::default(),
        }
    }
}

// This is our new error type. It has one variant for errors from the inner serializer, and one
// variant for AsStringErrors.
#[derive(Debug)]
enum MySerializerError<E> {
    Inner(E),
    AsStringError,
}

// This is the crux of our new error type. Since it implements From<AsStringError>, we'll be able to
// use our serializer with the AsString wrapper.
impl<E> From<AsStringError> for MySerializerError<E> {
    fn from(_: AsStringError) -> Self {
        Self::AsStringError
    }
}

fn main() {
    // Here, we make a simple local path to some foo.txt
    let example = Example {
        path: Path::new("foo.txt").to_path_buf(),
    };

    // We can construct our serializer in much the same way as we always do
    let mut serializer = MySerializer::<AllocSerializer<1024>>::default();
    // then manually serialize our value
    serializer.serialize_value(&example).unwrap();
    // and finally, dig all the way down to our byte buffer
    let bytes = serializer.into_inner().into_serializer().into_inner();

    // With that done, we can access our path as if it were a string!
    let example = unsafe { archived_root::<Example>(&bytes) };
    println!("The path is {}", example.path);
}
