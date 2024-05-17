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

impl From<ArchivedDuration> for Duration {
    #[inline]
    fn from(duration: ArchivedDuration) -> Self {
        Self::new(duration.as_secs(), duration.subsec_nanos())
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use crate::test::roundtrip;

    #[test]
    fn roundtrip_duration() {
        roundtrip(&Duration::new(1234, 5678));
    }
}
