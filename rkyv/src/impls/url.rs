use crate::net::{ArchivedIpv4Addr, ArchivedIpv6Addr};
use crate::{Archive, Archived, Deserialize, Fallible, Serialize};
use url::Url;

impl Archive for Url {
    type Archived = ArchivedUrl;
    type Resolver = ();

    unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: *mut Self::Archived) {
        unimplemented!()
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for Url {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<Url, D> for Url {
    fn deserialize(&self, _: &mut D) -> Result<Url, D::Error> {
        Ok(self.to_owned())
    }
}

/// A parsed URL record.
pub struct ArchivedUrl {
    /// Syntax in pseudo-BNF:
    ///
    ///   url = scheme ":" [ hierarchical | non-hierarchical ] [ "?" query ]? [ "#" fragment ]?
    ///   non-hierarchical = non-hierarchical-path
    ///   non-hierarchical-path = /* Does not start with "/" */
    ///   hierarchical = authority? hierarchical-path
    ///   authority = "//" userinfo? host [ ":" port ]?
    ///   userinfo = username [ ":" password ]? "@"
    ///   hierarchical-path = [ "/" path-segment ]+
    serialization: Archived<String>,

    // Components
    scheme_end: Archived<u32>,   // Before ':'
    username_end: Archived<u32>, // Before ':' (if a password is given) or '@' (if not)
    host_start: Archived<u32>,
    host_end: Archived<u32>,
    host: ArchivedHostInternal,
    port: Archived<Option<u16>>,
    path_start: Archived<u32>,             // Before initial '/', if any
    query_start: Archived<Option<u32>>,    // Before '?', unlike Position::QueryStart
    fragment_start: Archived<Option<u32>>, // Before '#', unlike Position::FragmentStart
}

pub enum ArchivedHostInternal {
    None,
    Domain,
    Ipv4(ArchivedIpv4Addr),
    Ipv6(ArchivedIpv6Addr),
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
