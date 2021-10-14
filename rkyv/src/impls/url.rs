use crate::out_field;
use crate::{Archive, Deserialize, Serialize};
use crate::{Archived, Fallible, Resolver};
use core::mem::MaybeUninit;
use std::io::Write;
use std::net::{Ipv4Addr, Ipv6Addr};
use url::Url;

impl Archive for Url {
    type Archived = ArchivedUrl;
    type Resolver = ArchivedUrlResolver;

    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        let mut result = MaybeUninit::<Url>::zeroed();
        let url = result.as_mut_ptr();
        macro_rules! resolve_foreach_field {
            ($($field:ident),+)=> {
                $(
                    let (fp, fo) = out_field!(url.$field);
                    out.$field.resolve(pos + fp, resolver.$field, fo);
                )+
            };
        }
        resolve_foreach_field![
            serialization,
            scheme_end,
            username_end,
            host_start,
            host_end,
            host,
            port,
            path_start,
            query_start,
            fragment_start
        ];
    }
}

#[rustfmt::skip]
impl<S: Fallible + ?Sized> Serialize<S> for Url {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(Self::Resolver {
            serialization: Serialize::<S>::serialize(&self.serialization, serializer)?,
            scheme_end: Serialize::<S>::serialize(&self.scheme_end, serializer)?,
            username_end: Serialize::<S>::serialize(&self.username_end, serializer)?,
            host_start: Serialize::<S>::serialize(&self.host_start, serializer)?,
            host_end: Serialize::<S>::serialize(&self.host_end, serializer)?,
            host: Serialize::<S>::serialize(&self.host, serializer)?,
            port: Serialize::<S>::serialize(&self.port, serializer)?,
            path_start: Serialize::<S>::serialize(&self.path_start, serializer)?,
            query_start: Serialize::<S>::serialize(&self.query_start, serializer)?,
            fragment_start: Serialize::<S>::serialize(&self.fragment_start, serializer)?,
        })
    }
}

#[rustfmt::skip]
impl<D: Fallible + ?Sized> Deserialize<Url, D> for Url {
    fn deserialize(&self, deserializer: &mut D) -> Result<Url, D::Error> {
        Ok(Self {
            serialization: Deserialize::<String, D>::deserialize(&self.serialization, deserializer)?,
            scheme_end: Deserialize::<u32, D>::deserialize(&self.scheme_end, deserializer)?,
            username_end: Deserialize::<u32, D>::deserialize(&self.username_end, deserializer)?,
            host_start: Deserialize::<u32, D>::deserialize(&self.host_start, deserializer)?,
            host_end: Deserialize::<u32, D>::deserialize(&self.host_end, deserializer)?,
            host: Deserialize::<HostInternal, D>::deserialize(&self.host, deserializer)?,
            port: Deserialize::<Option<u16>, D>::deserialize(&self.port, deserializer)?,
            path_start: Deserialize::<u32, D>::deserialize(&self.path_start, deserializer)?,
            query_start: Deserialize::<Option<u32>, D>::deserialize(&self.query_start, deserializer)?,
            fragment_start: Deserialize::<Option<u32>, D>::deserialize(&self.fragment_start, deserializer)?,
        })
    }
}

/// A parsed URL record.
pub struct ArchivedUrl {
    /// Syntax in pseudo-BNF:
    ///
    /// - url = scheme ":" [ hierarchical | non-hierarchical ] [ "?" query ]? [ "#" fragment ]?
    /// - non-hierarchical = non-hierarchical-path
    /// - non-hierarchical-path = /* Does not start with "/" */
    /// - hierarchical = authority? hierarchical-path
    /// - authority = "//" userinfo? host [ ":" port ]?
    /// - userinfo = username [ ":" password ]? "@"
    /// - hierarchical-path = [ "/" path-segment ]+
    serialization: Archived<String>,

    // Components
    scheme_end: Archived<u32>,
    // Before ':'
    username_end: Archived<u32>,
    // Before ':' (if a password is given) or '@' (if not)
    host_start: Archived<u32>,
    host_end: Archived<u32>,
    host: Archived<HostInternal>,
    port: Archived<Option<u16>>,
    path_start: Archived<u32>,
    // Before initial '/', if any
    query_start: Archived<Option<u32>>,
    // Before '?', unlike Position::QueryStart
    fragment_start: Archived<Option<u32>>, // Before '#', unlike Position::FragmentStart
}

pub struct ArchivedUrlResolver {
    serialization: Resolver<String>,
    scheme_end: Resolver<u32>,
    username_end: Resolver<u32>,
    host_start: Resolver<u32>,
    host_end: Resolver<u32>,
    host: Resolver<HostInternal>,
    port: Resolver<Option<u16>>,
    path_start: Resolver<u32>,
    query_start: Resolver<Option<u32>>,
    fragment_start: Resolver<Option<u32>>,
}

#[derive(Archive, Serialize, Deserialize)]
pub enum HostInternal {
    None,
    Domain,
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
}

#[cfg(test)]
mod rkyv_tests {
    use crate::{
        archived_root,
        ser::{serializers::AlignedSerializer, Serializer},
        util::AlignedVec,
        Deserialize, Infallible,
    };
    use std::str::FromStr;
    use url::Url;

    #[test]
    fn test_serialize_deserialize() {
        let url_str = "file://example/path";
        let u = Url::from_str(url_str).unwrap();

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer
            .serialize_value(&u)
            .expect("failed to archive Url");
        let buf = serializer.into_inner();
        let archived = unsafe { archived_root::<Url>(buf.as_ref()) };

        assert_eq!(&u, archived);

        let deserialized = archived
            .deserialize(&mut Infallible)
            .expect("failed to deserialize Url");

        assert_eq!(u, deserialized);
    }
}
