use crate::db::{Db, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::time::Duration;

const RDB_HEADER: &[u8] = b"REDIS_CLONE_RDB_V1";
const TYPE_STRING: u8 = 1;
const TYPE_LIST: u8 = 2;
const TYPE_HASH: u8 = 3;
const TYPE_SET: u8 = 4;
const TYPE_ZSET: u8 = 5;
const EOF_TAG: u8 = 0xFF;

fn write_u32<W: Write>(w: &mut W, val: u32) -> std::io::Result<()> {
    w.write_all(&val.to_be_bytes())
}

fn read_u32<R: Read>(r: &mut R) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn write_u64<W: Write>(w: &mut W, val: u64) -> std::io::Result<()> {
    w.write_all(&val.to_be_bytes())
}

fn read_u64<R: Read>(r: &mut R) -> std::io::Result<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_be_bytes(buf))
}

fn write_f64<W: Write>(w: &mut W, val: f64) -> std::io::Result<()> {
    w.write_all(&val.to_be_bytes())
}

fn read_f64<R: Read>(r: &mut R) -> std::io::Result<f64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(f64::from_be_bytes(buf))
}

fn write_bytes<W: Write>(w: &mut W, bytes: &[u8]) -> std::io::Result<()> {
    write_u32(w, bytes.len() as u32)?;
    w.write_all(bytes)
}

fn read_bytes<R: Read>(r: &mut R) -> std::io::Result<Vec<u8>> {
    let len = read_u32(r)? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn dump_db(db: &Db, path: &str) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    w.write_all(RDB_HEADER)?;

    let snapshot = db.snapshot_entries();

    for (key, val, expiry) in snapshot {
        match val {
            Value::String(data) => {
                w.write_all(&[TYPE_STRING])?;
                write_bytes(&mut w, &key)?;
                write_expiry(&mut w, expiry)?;
                write_bytes(&mut w, &data)?;
            }
            Value::List(list) => {
                w.write_all(&[TYPE_LIST])?;
                write_bytes(&mut w, &key)?;
                write_expiry(&mut w, expiry)?;
                write_u32(&mut w, list.len() as u32)?;
                for elem in list {
                    write_bytes(&mut w, &elem)?;
                }
            }
            Value::Hash(hash) => {
                w.write_all(&[TYPE_HASH])?;
                write_bytes(&mut w, &key)?;
                write_expiry(&mut w, expiry)?;
                write_u32(&mut w, hash.len() as u32)?;
                for (f, v) in hash {
                    write_bytes(&mut w, &f)?;
                    write_bytes(&mut w, &v)?;
                }
            }
            Value::Set(set) => {
                w.write_all(&[TYPE_SET])?;
                write_bytes(&mut w, &key)?;
                write_expiry(&mut w, expiry)?;
                write_u32(&mut w, set.len() as u32)?;
                for member in set {
                    write_bytes(&mut w, &member)?;
                }
            }
            Value::ZSet(zset) => {
                w.write_all(&[TYPE_ZSET])?;
                write_bytes(&mut w, &key)?;
                write_expiry(&mut w, expiry)?;
                write_u32(&mut w, zset.dict.len() as u32)?;
                for (member, &score) in &zset.dict {
                    write_bytes(&mut w, member)?;
                    write_f64(&mut w, score)?;
                }
            }
        }
    }

    w.write_all(&[EOF_TAG])?;
    w.flush()
}

fn write_expiry<W: Write>(w: &mut W, expiry: Option<Duration>) -> std::io::Result<()> {
    match expiry {
        Some(dur) => {
            w.write_all(&[1])?;
            write_u64(w, dur.as_millis() as u64)
        }
        None => w.write_all(&[0]),
    }
}

pub fn load_db(path: &str) -> std::io::Result<Db> {
    let file = File::open(path)?;
    let mut r = BufReader::new(file);

    let mut header = [0u8; 18];
    r.read_exact(&mut header)?;
    if header != RDB_HEADER {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid RDB header",
        ));
    }

    let mut db = Db::new();

    loop {
        let mut tag_buf = [0u8; 1];
        if r.read(&mut tag_buf)? == 0 {
            break;
        }
        let tag = tag_buf[0];
        if tag == EOF_TAG {
            break;
        }

        let key = read_bytes(&mut r)?;

        let mut flag_buf = [0u8; 1];
        r.read_exact(&mut flag_buf)?;
        let expiry = if flag_buf[0] == 1 {
            let ms = read_u64(&mut r)?;
            Some(Duration::from_millis(ms))
        } else {
            None
        };

        match tag {
            TYPE_STRING => {
                let data = read_bytes(&mut r)?;
                db.set(key.clone(), Value::String(data));
            }
            TYPE_LIST => {
                let count = read_u32(&mut r)? as usize;
                let mut list = VecDeque::with_capacity(count);
                for _ in 0..count {
                    list.push_back(read_bytes(&mut r)?);
                }
                db.set(key.clone(), Value::List(list));
            }
            TYPE_HASH => {
                let count = read_u32(&mut r)? as usize;
                let mut hash = HashMap::with_capacity(count);
                for _ in 0..count {
                    let f = read_bytes(&mut r)?;
                    let v = read_bytes(&mut r)?;
                    hash.insert(f, v);
                }
                db.set(key.clone(), Value::Hash(hash));
            }
            TYPE_SET => {
                let count = read_u32(&mut r)? as usize;
                let mut set = HashSet::with_capacity(count);
                for _ in 0..count {
                    set.insert(read_bytes(&mut r)?);
                }
                db.set(key.clone(), Value::Set(set));
            }
            TYPE_ZSET => {
                let count = read_u32(&mut r)? as usize;
                let mut pairs = Vec::with_capacity(count);
                for _ in 0..count {
                    let member = read_bytes(&mut r)?;
                    let score = read_f64(&mut r)?;
                    pairs.push((score, member));
                }
                db.zadd(&key, &pairs).ok();
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Unknown RDB tag",
                ))
            }
        }

        if let Some(dur) = expiry {
            db.set_expire(&key, dur);
        }
    }

    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdb_roundtrip() {
        let mut db = Db::new();
        db.set(b"str".to_vec(), Value::String(b"hello".to_vec()));
        db.rpush(b"lst", &[b"a".to_vec(), b"b".to_vec()]).ok();
        db.hset(b"hsh", &[(b"f1".to_vec(), b"v1".to_vec())]).ok();
        db.sadd(b"st", &[b"m1".to_vec()]).ok();
        db.zadd(b"zst", &[(10.5, b"z1".to_vec())]).ok();
        db.set_expire(b"str", Duration::from_secs(600));

        let temp_path = "test_dump.rdb";
        dump_db(&db, temp_path).unwrap();

        let mut loaded = load_db(temp_path).unwrap();
        assert_eq!(loaded.get(b"str"), Some(&Value::String(b"hello".to_vec())));
        assert_eq!(loaded.lrange(b"lst", 0, -1).unwrap().len(), 2);
        assert_eq!(loaded.hget(b"hsh", b"f1").unwrap(), Some(b"v1".to_vec()));
        assert_eq!(loaded.sismember(b"st", b"m1").unwrap(), true);
        assert_eq!(loaded.zscore(b"zst", b"z1").unwrap(), Some(10.5));
        assert!(loaded.ttl(b"str") > 0);

        std::fs::remove_file(temp_path).ok();
    }
}
