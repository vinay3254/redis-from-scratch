use crate::skiplist::SkipList;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Clone)]
pub struct ZSet {
    pub dict: HashMap<Vec<u8>, f64>,
    pub skiplist: SkipList,
}

impl ZSet {
    pub fn new() -> Self {
        ZSet {
            dict: HashMap::new(),
            skiplist: SkipList::new(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    String(Vec<u8>),
    List(VecDeque<Vec<u8>>),
    Hash(HashMap<Vec<u8>, Vec<u8>>),
    Set(HashSet<Vec<u8>>),
    ZSet(ZSet),
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

    pub fn snapshot_entries(&self) -> Vec<(Vec<u8>, Value, Option<Duration>)> {
        let now = Instant::now();
        let mut snapshot = Vec::new();
        for (k, v) in &self.entries {
            if let Some(&expire_at) = self.expirations.get(k) {
                if now >= expire_at {
                    continue;
                }
                let remaining = expire_at - now;
                snapshot.push((k.clone(), v.clone(), Some(remaining)));
            } else {
                snapshot.push((k.clone(), v.clone(), None));
            }
        }
        snapshot
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

    pub fn hset(&mut self, key: &[u8], pairs: &[(Vec<u8>, Vec<u8>)]) -> Result<usize, ()> {
        self.check_expired(key);
        let hash = match self.entries.get_mut(key) {
            Some(Value::Hash(h)) => h,
            Some(_) => return Err(()),
            None => {
                self.entries
                    .insert(key.to_vec(), Value::Hash(HashMap::new()));
                match self.entries.get_mut(key) {
                    Some(Value::Hash(h)) => h,
                    _ => unreachable!(),
                }
            }
        };
        let mut created = 0;
        for (field, val) in pairs {
            if hash.insert(field.clone(), val.clone()).is_none() {
                created += 1;
            }
        }
        Ok(created)
    }

    pub fn hget(&mut self, key: &[u8], field: &[u8]) -> Result<Option<Vec<u8>>, ()> {
        self.check_expired(key);
        match self.entries.get(key) {
            Some(Value::Hash(h)) => Ok(h.get(field).cloned()),
            Some(_) => Err(()),
            None => Ok(None),
        }
    }

    pub fn hgetall(&mut self, key: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, ()> {
        self.check_expired(key);
        match self.entries.get(key) {
            Some(Value::Hash(h)) => Ok(h.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            Some(_) => Err(()),
            None => Ok(Vec::new()),
        }
    }

    pub fn hdel(&mut self, key: &[u8], fields: &[Vec<u8>]) -> Result<usize, ()> {
        self.check_expired(key);
        match self.entries.get_mut(key) {
            Some(Value::Hash(h)) => {
                let mut removed = 0;
                for field in fields {
                    if h.remove(field).is_some() {
                        removed += 1;
                    }
                }
                if h.is_empty() {
                    self.entries.remove(key);
                    self.expirations.remove(key);
                }
                Ok(removed)
            }
            Some(_) => Err(()),
            None => Ok(0),
        }
    }

    pub fn sadd(&mut self, key: &[u8], members: &[Vec<u8>]) -> Result<usize, ()> {
        self.check_expired(key);
        let set = match self.entries.get_mut(key) {
            Some(Value::Set(s)) => s,
            Some(_) => return Err(()),
            None => {
                self.entries
                    .insert(key.to_vec(), Value::Set(HashSet::new()));
                match self.entries.get_mut(key) {
                    Some(Value::Set(s)) => s,
                    _ => unreachable!(),
                }
            }
        };
        let mut added = 0;
        for member in members {
            if set.insert(member.clone()) {
                added += 1;
            }
        }
        Ok(added)
    }

    pub fn srem(&mut self, key: &[u8], members: &[Vec<u8>]) -> Result<usize, ()> {
        self.check_expired(key);
        match self.entries.get_mut(key) {
            Some(Value::Set(s)) => {
                let mut removed = 0;
                for member in members {
                    if s.remove(member) {
                        removed += 1;
                    }
                }
                if s.is_empty() {
                    self.entries.remove(key);
                    self.expirations.remove(key);
                }
                Ok(removed)
            }
            Some(_) => Err(()),
            None => Ok(0),
        }
    }

    pub fn smembers(&mut self, key: &[u8]) -> Result<Vec<Vec<u8>>, ()> {
        self.check_expired(key);
        match self.entries.get(key) {
            Some(Value::Set(s)) => Ok(s.iter().cloned().collect()),
            Some(_) => Err(()),
            None => Ok(Vec::new()),
        }
    }

    pub fn sismember(&mut self, key: &[u8], member: &[u8]) -> Result<bool, ()> {
        self.check_expired(key);
        match self.entries.get(key) {
            Some(Value::Set(s)) => Ok(s.contains(member)),
            Some(_) => Err(()),
            None => Ok(false),
        }
    }

    pub fn zadd(&mut self, key: &[u8], pairs: &[(f64, Vec<u8>)]) -> Result<usize, ()> {
        self.check_expired(key);
        let zset = match self.entries.get_mut(key) {
            Some(Value::ZSet(z)) => z,
            Some(_) => return Err(()),
            None => {
                self.entries
                    .insert(key.to_vec(), Value::ZSet(ZSet::new()));
                match self.entries.get_mut(key) {
                    Some(Value::ZSet(z)) => z,
                    _ => unreachable!(),
                }
            }
        };
        let mut added = 0;
        for (score, member) in pairs {
            if let Some(&old_score) = zset.dict.get(member) {
                zset.skiplist.remove(old_score, member);
                zset.dict.insert(member.clone(), *score);
                zset.skiplist.insert(*score, member.clone());
            } else {
                zset.dict.insert(member.clone(), *score);
                zset.skiplist.insert(*score, member.clone());
                added += 1;
            }
        }
        Ok(added)
    }

    pub fn zscore(&mut self, key: &[u8], member: &[u8]) -> Result<Option<f64>, ()> {
        self.check_expired(key);
        match self.entries.get(key) {
            Some(Value::ZSet(z)) => Ok(z.dict.get(member).copied()),
            Some(_) => Err(()),
            None => Ok(None),
        }
    }

    pub fn zrange(
        &mut self,
        key: &[u8],
        start: i64,
        stop: i64,
    ) -> Result<Vec<(Vec<u8>, f64)>, ()> {
        self.check_expired(key);
        match self.entries.get(key) {
            Some(Value::ZSet(z)) => {
                match normalize_indices(z.skiplist.len(), start, stop) {
                    Some((s, e)) => Ok(z.skiplist.get_range(s, e)),
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

    #[test]
    fn test_hash_operations() {
        let mut db = Db::new();
        let k = b"h1";

        let pairs = vec![
            (b"f1".to_vec(), b"v1".to_vec()),
            (b"f2".to_vec(), b"v2".to_vec()),
        ];
        assert_eq!(db.hset(k, &pairs), Ok(2));
        assert_eq!(db.hget(k, b"f1"), Ok(Some(b"v1".to_vec())));
        assert_eq!(db.hget(k, b"f3"), Ok(None));

        assert_eq!(db.hdel(k, &[b"f1".to_vec()]), Ok(1));
        assert_eq!(db.hget(k, b"f1"), Ok(None));
    }

    #[test]
    fn test_set_operations() {
        let mut db = Db::new();
        let k = b"s1";

        assert_eq!(db.sadd(k, &[b"m1".to_vec(), b"m2".to_vec()]), Ok(2));
        assert_eq!(db.sadd(k, &[b"m1".to_vec()]), Ok(0));
        assert_eq!(db.sismember(k, b"m1"), Ok(true));
        assert_eq!(db.sismember(k, b"m3"), Ok(false));

        assert_eq!(db.srem(k, &[b"m1".to_vec()]), Ok(1));
        assert_eq!(db.sismember(k, b"m1"), Ok(false));
    }

    #[test]
    fn test_zset_operations() {
        let mut db = Db::new();
        let k = b"z1";

        let pairs = vec![(10.0, b"one".to_vec()), (20.0, b"two".to_vec())];
        assert_eq!(db.zadd(k, &pairs), Ok(2));
        assert_eq!(db.zscore(k, b"two"), Ok(Some(20.0)));

        let range = db.zrange(k, 0, -1).unwrap();
        assert_eq!(
            range,
            vec![(b"one".to_vec(), 10.0), (b"two".to_vec(), 20.0)]
        );
    }
}
