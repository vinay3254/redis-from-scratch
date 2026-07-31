use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    String(Vec<u8>),
}

pub struct Db {
    entries: HashMap<Vec<u8>, Value>,
    expirations: HashMap<Vec<u8>, Instant>,
}

impl Db {
    pub fn new() -> Self {
        Db {
            entries: HashMap::new(),
            expirations: HashMap::new(),
        }
    }

    fn check_expired(&mut self, key: &[u8]) -> bool {
        if let Some(&expire_at) = self.expirations.get(key) {
            if Instant::now() >= expire_at {
                self.entries.remove(key);
                self.expirations.remove(key);
                return true;
            }
        }
        false
    }

    pub fn get(&mut self, key: &[u8]) -> Option<&Value> {
        if self.check_expired(key) {
            return None;
        }
        self.entries.get(key)
    }

    pub fn set(&mut self, key: Vec<u8>, value: Value) {
        self.expirations.remove(&key);
        self.entries.insert(key, value);
    }

    pub fn del(&mut self, keys: &[Vec<u8>]) -> usize {
        let mut count = 0;
        for key in keys {
            self.check_expired(key);
            self.expirations.remove(key);
            if self.entries.remove(key).is_some() {
                count += 1;
            }
        }
        count
    }

    pub fn exists(&mut self, keys: &[Vec<u8>]) -> usize {
        let mut count = 0;
        for key in keys {
            if !self.check_expired(key) && self.entries.contains_key(key) {
                count += 1;
            }
        }
        count
    }

    pub fn set_expire(&mut self, key: &[u8], duration: Duration) -> bool {
        if self.check_expired(key) || !self.entries.contains_key(key) {
            return false;
        }
        self.expirations
            .insert(key.to_vec(), Instant::now() + duration);
        true
    }

    pub fn ttl(&mut self, key: &[u8]) -> i64 {
        if self.check_expired(key) || !self.entries.contains_key(key) {
            return -2;
        }
        match self.expirations.get(key) {
            Some(&expire_at) => {
                let now = Instant::now();
                if now >= expire_at {
                    self.entries.remove(key);
                    self.expirations.remove(key);
                    -2
                } else {
                    (expire_at - now).as_secs() as i64
                }
            }
            None => -1,
        }
    }

    pub fn purge_expired(&mut self) -> usize {
        let now = Instant::now();
        let expired_keys: Vec<Vec<u8>> = self
            .expirations
            .iter()
            .filter_map(|(k, &expire_at)| if now >= expire_at { Some(k.clone()) } else { None })
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            self.entries.remove(&key);
            self.expirations.remove(&key);
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

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

    #[test]
    fn test_expiry() {
        let mut db = Db::new();
        db.set(b"k1".to_vec(), Value::String(b"v1".to_vec()));

        assert_eq!(db.ttl(b"k1"), -1);
        assert_eq!(db.set_expire(b"k1", Duration::from_millis(50)), true);

        assert_eq!(db.exists(&[b"k1".to_vec()]), 1);

        sleep(Duration::from_millis(60));

        assert_eq!(db.ttl(b"k1"), -2);
        assert_eq!(db.get(b"k1"), None);
        assert_eq!(db.exists(&[b"k1".to_vec()]), 0);
    }

    #[test]
    fn test_purge_expired() {
        let mut db = Db::new();
        db.set(b"k1".to_vec(), Value::String(b"v1".to_vec()));
        db.set_expire(b"k1", Duration::from_millis(30));

        sleep(Duration::from_millis(40));

        assert_eq!(db.purge_expired(), 1);
        assert_eq!(db.ttl(b"k1"), -2);
    }
}
