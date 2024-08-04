use core::time::Duration;

use rancor::Fallible;

use crate::{time::ArchivedDuration, Archive, Deserialize, Place, Serialize};

impl Archive for Duration {
    type Archived = ArchivedDuration;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        unsafe {
            ArchivedDuration::emplace(
                self.as_secs(),
                self.subsec_nanos(),
                out.ptr(),
            );
        }
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for Duration {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<Duration, D> for ArchivedDuration {
    fn deserialize(&self, _: &mut D) -> Result<Duration, D::Error> {
        Ok(Duration::new(self.as_secs(), self.subsec_nanos()))
    }
}

impl PartialEq<Duration> for ArchivedDuration {
    #[inline]
    fn eq(&self, other: &Duration) -> bool {
        self.as_nanos() == other.as_nanos() && self.as_secs() == other.as_secs()
    }
}

impl PartialEq<ArchivedDuration> for Duration {
    #[inline]
    fn eq(&self, other: &ArchivedDuration) -> bool {
        other.eq(self)
    }
}

impl From<ArchivedDuration> for Duration {
    #[inline]
    fn from(duration: ArchivedDuration) -> Self {
        Self::new(duration.as_secs(), duration.subsec_nanos())
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use rancor::Failure;

    use crate::{api::test::roundtrip, from_bytes, util::Align};

    #[test]
    fn roundtrip_duration() {
        roundtrip(&Duration::new(1234, 5678));
    }

    // TODO: this should test we catch cases where nanos > 1_000_000_000
    #[test]
    fn invalid_duration() {
        let data = Align([0xff, 0x10]);
        from_bytes::<Duration, Failure>(&*data).unwrap_err();
    }
}
