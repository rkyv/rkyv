#[cfg(test)]
mod tests {
    use crate::validation::util::serialize_and_check;
    use std::collections::{HashMap, HashSet};

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn hashmap() {
        let mut map = HashMap::new();
        map.insert("Hello".to_string(), 12);
        map.insert("world".to_string(), 34);
        map.insert("foo".to_string(), 56);
        map.insert("bar".to_string(), 78);
        map.insert("baz".to_string(), 90);
        serialize_and_check(&map);

        let mut set = HashSet::new();
        set.insert("Hello".to_string());
        set.insert("world".to_string());
        set.insert("foo".to_string());
        set.insert("bar".to_string());
        set.insert("baz".to_string());
        serialize_and_check(&set);
    }
}
