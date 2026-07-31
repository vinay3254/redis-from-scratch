use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    String(Vec<u8>),
}

pub struct Db {
    entries: HashMap<Vec<u8>, Value>,
}

impl Db {
    pub fn new() -> Self {
        Db {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&Value> {
        self.entries.get(key)
    }

    pub fn set(&mut self, key: Vec<u8>, value: Value) {
        self.entries.insert(key, value);
    }

    pub fn del(&mut self, keys: &[Vec<u8>]) -> usize {
        let mut count = 0;
        for key in keys {
            if self.entries.remove(key).is_some() {
                count += 1;
            }
        }
        count
    }

    pub fn exists(&self, keys: &[Vec<u8>]) -> usize {
        let mut count = 0;
        for key in keys {
            if self.entries.contains_key(key) {
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_operations() {
        let mut db = Db::new();
        db.set(b"key1".to_vec(), Value::String(b"val1".to_vec()));

        assert_eq!(db.get(b"key1"), Some(&Value::String(b"val1".to_vec())));
        assert_eq!(db.get(b"key2"), None);

        assert_eq!(db.exists(&[b"key1".to_vec(), b"key2".to_vec()]), 1);

        assert_eq!(db.del(&[b"key1".to_vec(), b"key2".to_vec()]), 1);
        assert_eq!(db.get(b"key1"), None);
    }
}
