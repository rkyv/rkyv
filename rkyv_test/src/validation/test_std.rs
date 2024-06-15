#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use rkyv::rancor::Error;

    use crate::validation::util::alloc::serialize_and_check;

    #[test]
    fn hashmap() {
        let mut map = HashMap::new();
        map.insert("Hello".to_string(), 12);
        map.insert("world".to_string(), 34);
        map.insert("foo".to_string(), 56);
        map.insert("bar".to_string(), 78);
        map.insert("baz".to_string(), 90);
        serialize_and_check::<_, Error>(&map);

        let mut set = HashSet::new();
        set.insert("Hello".to_string());
        set.insert("world".to_string());
        set.insert("foo".to_string());
        set.insert("bar".to_string());
        set.insert("baz".to_string());
        serialize_and_check::<_, Error>(&set);
    }
}
