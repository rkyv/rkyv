use bytes::BytesMut;
use postgres_types::{to_sql_checked, IsNull, ToSql, Type};
use std::{collections::HashMap, error::Error, hash::Hasher, net::IpAddr};

use crate::{
    boxed::ArchivedBox, collections::swiss_table::ArchivedHashMap,
    net::ArchivedIpAddr, niche::option_box::ArchivedOptionBox,
    option::ArchivedOption, rc::ArchivedRc, string::ArchivedString,
    vec::ArchivedVec,
};

macro_rules! fwd_accepts {
    ($ty:ty) => {
        #[inline]
        fn accepts(ty: &Type) -> bool {
            <$ty as ToSql>::accepts(ty)
        }
    };
}

impl ToSql for ArchivedString {
    #[inline]
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        self.as_str().to_sql(ty, out)
    }

    fwd_accepts!(&str);
    to_sql_checked!();
}

impl<T> ToSql for ArchivedVec<T>
where
    T: ToSql,
{
    #[inline]
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        self.as_slice().to_sql(ty, out)
    }

    fwd_accepts!(&[T]);
    to_sql_checked!();
}

impl<T> ToSql for ArchivedOption<T>
where
    T: ToSql,
{
    #[inline]
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        match self {
            ArchivedOption::Some(value) => value.to_sql(ty, out),
            ArchivedOption::None => Ok(IsNull::Yes),
        }
    }

    fwd_accepts!(Option<T>);
    to_sql_checked!();
}

impl<T> ToSql for ArchivedBox<T>
where
    T: ToSql,
{
    #[inline]
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        self.as_ref().to_sql(ty, out)
    }

    fwd_accepts!(T);
    to_sql_checked!();
}

impl<T> ToSql for ArchivedOptionBox<T>
where
    T: ToSql,
{
    #[inline]
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        match self.as_ref() {
            Some(value) => value.to_sql(ty, out),
            None => Ok(IsNull::Yes),
        }
    }

    fwd_accepts!(Option<T>);
    to_sql_checked!();
}

impl<T, F> ToSql for ArchivedRc<T, F>
where
    T: ToSql,
{
    #[inline]
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        self.as_ref().to_sql(ty, out)
    }

    fwd_accepts!(T);
    to_sql_checked!();
}

impl ToSql for ArchivedIpAddr {
    #[inline]
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        self.as_ipaddr().to_sql(ty, out)
    }

    fwd_accepts!(IpAddr);
    to_sql_checked!();
}

impl<H> ToSql
    for ArchivedHashMap<ArchivedString, ArchivedOption<ArchivedString>, H>
where
    H: Hasher,
{
    #[inline]
    fn to_sql(
        &self,
        _ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        postgres_protocol::types::hstore_to_sql(
            self.iter()
                .map(|(k, v)| (k.as_ref(), v.as_ref().map(|v| v.as_ref()))),
            out,
        )?;

        Ok(IsNull::No)
    }

    fwd_accepts!(HashMap<String, Option<String>>);
    to_sql_checked!();
}
