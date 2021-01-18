// use bytecheck::CheckBytes;
// use super::ArchivedRc;
// use crate::validation::{SharedArchiveContext, SharedArchiveError};

// impl<T: CheckBytes<C>, C: SharedArchiveContext> CheckBytes<C> for ArchivedRc<T> {
//     type Error = SharedArchiveError;

//     unsafe fn check_bytes<'a>(bytes: *const u8, context: &mut C) -> Result<&'a Self, Self::Error> {
//         context.claim_shared::<T>()
//     }
// }