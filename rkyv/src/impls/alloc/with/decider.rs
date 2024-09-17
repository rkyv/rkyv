use core::ops::Deref;

use crate::{
    alloc::boxed::Box,
    boxed::BoxResolver,
    niche::{
        decider::{Decider, Null},
        option_box::{ArchivedOptionBox, OptionBoxResolver},
    },
    ArchiveUnsized, Archived, Place,
};

impl<T> Decider<Box<T>> for Null
where
    T: ArchiveUnsized + ?Sized,
{
    type Archived = ArchivedOptionBox<T::Archived>;

    fn as_option(archived: &Self::Archived) -> Option<&Archived<Box<T>>> {
        archived.as_ref()
    }

    fn resolve_from_option(
        option: Option<&Box<T>>,
        resolver: Option<BoxResolver>,
        out: Place<Self::Archived>,
    ) {
        let resolver =
            resolver.map_or(OptionBoxResolver::None, OptionBoxResolver::Some);

        ArchivedOptionBox::resolve_from_option(
            option.map(Box::deref),
            resolver,
            out,
        );
    }
}
