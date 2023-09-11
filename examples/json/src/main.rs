use rkyv::{
    check_archived_root,
    ser::{serializers::AllocSerializer, Serializer},
    Archive, Deserialize, Serialize,
};
use std::{collections::HashMap, fmt};

#[derive(Archive, Debug, Deserialize, Serialize)]
// We have a recursive type, which requires some special handling
//
// First the compiler will return an error:
//
// > error[E0275]: overflow evaluating the requirement `HashMap<String, JsonValue>: Archive`
//
// This is because the implementation of Archive for Json value requires that JsonValue: Archive,
//   which is recursive!
// We can fix this by adding #[omit_bounds] on the recursive fields. This will prevent the derive
//   from automatically adding a `HashMap<String, JsonValue>: Archive` bound on the generated impl.
//
// Next, the compiler will return these errors:
//
// > error[E0277]: the trait bound `__S: ScratchSpace` is not satisfied
// > error[E0277]: the trait bound `__S: Serializer` is not satisfied
//
// This is because those bounds are required by HashMap and Vec, but we removed the default
//   generated bounds to prevent a recursive impl.
// We can fix this by manually specifying the bounds required by HashMap and Vec in an attribute,
//   and then everything will compile:
#[archive(bound(
    serialize = "__S: rkyv::ser::ScratchSpace + rkyv::ser::Serializer"
))]
// We'll also add support for validating our archived type. Validation will allow us to check an
// arbitrary buffer of bytes before accessing it so we can avoid using any unsafe code.
//
// To validate our archived type, we also need to derive `CheckBytes` on it. We can use the
// `#[archive_attr(..)]` attribute to pass any attributes through to the generated type. So to
// derive `CheckBytes` for our archived type, we simply add `#[archive(check_bytes)]` to
// our type.
//
// This has the same issues as our `Archive` derive, so we need to follow a similar process:
//
// First, we need to add the same `omit_bounds` attribute to our fields. However, this time the
// attribute needs to be on the fields of the archived type. Luckily, we can pass through this
// attribute in the same way, using `#[archive_attr(omit_bounds)]`.
//
// Next, we need to manually add the appropriate non-recursive bounds to our type. In our case, we
// need to bound:
//
// `__C: rkyv::validation::ArchiveContext`: This will make sure that our `Vec` and `HashMap` have
// the `ArchiveContext` trait implemented on the validator. This is a necessary requirement for
// containers to check their bytes.
//
// `<__C as rkyv::Fallible>::Error: std::error::Error`: This bounds our validation context so that
// we know its error type implements `Error`. This is necessary when deriving `CheckBytes` since the
// error type for structs is a `bytecheck::StructCheckError` which contains the inner error as a
// `Box<dyn Error>`.
//
// With those two changes, our recursive type can be validated with `check_archived_root`!
#[archive(check_bytes)]
#[archive_attr(check_bytes(
    bound = "__C: rkyv::validation::ArchiveContext, <__C as rkyv::Fallible>::Error: rkyv::bytecheck::Error"
))]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(
        #[omit_bounds]
        #[archive_attr(omit_bounds)]
        Vec<JsonValue>,
    ),
    Object(
        #[omit_bounds]
        #[archive_attr(omit_bounds)]
        HashMap<String, JsonValue>,
    ),
}

impl fmt::Display for ArchivedJsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null")?,
            Self::Bool(b) => write!(f, "{}", b)?,
            Self::Number(n) => write!(f, "{}", n)?,
            Self::String(s) => write!(f, "{}", s)?,
            Self::Array(a) => {
                write!(f, "[")?;
                for (i, value) in a.iter().enumerate() {
                    write!(f, "{}", value)?;
                    if i < a.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "]")?;
            }
            Self::Object(h) => {
                write!(f, "{{")?;
                for (i, (key, value)) in h.iter().enumerate() {
                    write!(f, "\"{}\": {}", key, value)?;
                    if i < h.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "}}")?;
            }
        }
        Ok(())
    }
}

#[derive(Archive, Debug, Deserialize, Serialize)]
#[archive(check_bytes)]
pub enum JsonNumber {
    PosInt(u64),
    NegInt(i64),
    Float(f64),
}

impl fmt::Display for ArchivedJsonNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PosInt(n) => write!(f, "{}", n),
            Self::NegInt(n) => write!(f, "{}", n),
            Self::Float(n) => write!(f, "{}", n),
        }
    }
}

fn main() {
    let mut hash_map = HashMap::new();
    hash_map.insert("name".into(), JsonValue::String("ferris".into()));
    hash_map.insert("age".into(), JsonValue::Number(JsonNumber::PosInt(10)));
    hash_map.insert("is_crab".into(), JsonValue::Bool(true));
    hash_map.insert("project".into(), JsonValue::Null);
    let value = JsonValue::Object(hash_map);

    let mut serializer = AllocSerializer::<4096>::default();
    serializer.serialize_value(&value).unwrap();

    let buf = serializer.into_serializer().into_inner();
    let archived_value = check_archived_root::<JsonValue>(&buf).unwrap();

    println!("{}", archived_value);
}
