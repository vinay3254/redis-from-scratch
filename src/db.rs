use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    String(Vec<u8>),
    List(VecDeque<Vec<u8>>),
}

pub struct Db {
    entries: HashMap<Vec<u8>, Value>,
    expirations: HashMap<Vec<u8>, Instant>,
}

fn normalize_indices(len: usize, start: i64, stop: i64) -> Option<(usize, usize)> {
    if len == 0 {
        return None;
    }
    let l = len as i64;
    let mut s = if start < 0 { l + start } else { start };
    let mut e = if stop < 0 { l + stop } else { stop };

    if s < 0 {
        s = 0;
    }
    if e < 0 {
        return None;
    }

    if s >= l {
        return None;
    }
    if e >= l {
        e = l - 1;
    }

    if s > e {
        return None;
    }

    Some((s as usize, e as usize))
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

    pub fn lpush(&mut self, key: &[u8], elements: &[Vec<u8>]) -> Result<usize, ()> {
        self.check_expired(key);
        let list = match self.entries.get_mut(key) {
            Some(Value::List(l)) => l,
            Some(_) => return Err(()),
            None => {
                self.entries
                    .insert(key.to_vec(), Value::List(VecDeque::new()));
                match self.entries.get_mut(key) {
                    Some(Value::List(l)) => l,
                    _ => unreachable!(),
                }
            }
        };
        for elem in elements {
            list.push_front(elem.clone());
        }
        Ok(list.len())
    }

    pub fn rpush(&mut self, key: &[u8], elements: &[Vec<u8>]) -> Result<usize, ()> {
        self.check_expired(key);
        let list = match self.entries.get_mut(key) {
            Some(Value::List(l)) => l,
            Some(_) => return Err(()),
            None => {
                self.entries
                    .insert(key.to_vec(), Value::List(VecDeque::new()));
                match self.entries.get_mut(key) {
                    Some(Value::List(l)) => l,
                    _ => unreachable!(),
                }
            }
        };
        for elem in elements {
            list.push_back(elem.clone());
        }
        Ok(list.len())
    }

    pub fn lpop(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, ()> {
        self.check_expired(key);
        match self.entries.get_mut(key) {
            Some(Value::List(l)) => {
                let item = l.pop_front();
                if l.is_empty() {
                    self.entries.remove(key);
                    self.expirations.remove(key);
                }
                Ok(item)
            }
            Some(_) => Err(()),
            None => Ok(None),
        }
    }

    pub fn rpop(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, ()> {
        self.check_expired(key);
        match self.entries.get_mut(key) {
            Some(Value::List(l)) => {
                let item = l.pop_back();
                if l.is_empty() {
                    self.entries.remove(key);
                    self.expirations.remove(key);
                }
                Ok(item)
            }
            Some(_) => Err(()),
            None => Ok(None),
        }
    }

    pub fn lrange(&mut self, key: &[u8], start: i64, stop: i64) -> Result<Vec<Vec<u8>>, ()> {
        self.check_expired(key);
        match self.entries.get(key) {
            Some(Value::List(l)) => {
                match normalize_indices(l.len(), start, stop) {
                    Some((s, e)) => Ok(l.range(s..=e).cloned().collect()),
                    None => Ok(Vec::new()),
                }
            }
            Some(_) => Err(()),
            None => Ok(Vec::new()),
        }
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

    #[test]
    fn test_list_operations() {
        let mut db = Db::new();
        let k = b"l1";

        assert_eq!(db.rpush(k, &[b"a".to_vec(), b"b".to_vec()]), Ok(2));
        assert_eq!(db.lpush(k, &[b"first".to_vec()]), Ok(3));

        let res = db.lrange(k, 0, -1).unwrap();
        assert_eq!(res, vec![b"first".to_vec(), b"a".to_vec(), b"b".to_vec()]);

        assert_eq!(db.lpop(k), Ok(Some(b"first".to_vec())));
        assert_eq!(db.rpop(k), Ok(Some(b"b".to_vec())));
        assert_eq!(db.rpop(k), Ok(Some(b"a".to_vec())));
        assert_eq!(db.rpop(k), Ok(None));
    }
}
