use crate::db::Db;
use crate::persistence::rdb::dump_db;
use crate::resp::RespFrame;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn del(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.is_empty() {
        return RespFrame::Error("ERR wrong number of arguments for 'del' command".into());
    }
    let count = db.del(args);
    RespFrame::Integer(count as i64)
}

pub fn exists(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.is_empty() {
        return RespFrame::Error("ERR wrong number of arguments for 'exists' command".into());
    }
    let count = db.exists(args);
    RespFrame::Integer(count as i64)
}

pub fn expire(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'expire' command".into());
    }
    let seconds: u64 = match std::str::from_utf8(&args[1]).ok().and_then(|s| s.parse().ok()) {
        Some(s) => s,
        None => return RespFrame::Error("ERR value is not an integer or out of range".into()),
    };
    let success = db.set_expire(&args[0], Duration::from_secs(seconds));
    RespFrame::Integer(if success { 1 } else { 0 })
}

pub fn pexpire(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'pexpire' command".into());
    }
    let millis: u64 = match std::str::from_utf8(&args[1]).ok().and_then(|s| s.parse().ok()) {
        Some(s) => s,
        None => return RespFrame::Error("ERR value is not an integer or out of range".into()),
    };
    let success = db.set_expire(&args[0], Duration::from_millis(millis));
    RespFrame::Integer(if success { 1 } else { 0 })
}

pub fn ttl(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 1 {
        return RespFrame::Error("ERR wrong number of arguments for 'ttl' command".into());
    }
    let remaining = db.ttl(&args[0]);
    RespFrame::Integer(remaining)
}

pub fn save(db: &Db, args: &[Vec<u8>]) -> RespFrame {
    if !args.is_empty() {
        return RespFrame::Error("ERR wrong number of arguments for 'save' command".into());
    }
    match dump_db(db, "dump.rdb") {
        Ok(_) => RespFrame::SimpleString("OK".into()),
        Err(e) => RespFrame::Error(format!("ERR failed to save snapshot: {}", e)),
    }
}

pub fn bgsave(db: Arc<Mutex<Db>>, args: &[Vec<u8>]) -> RespFrame {
    if !args.is_empty() {
        return RespFrame::Error("ERR wrong number of arguments for 'bgsave' command".into());
    }
    thread::spawn(move || {
        let db_guard = db.lock().unwrap();
        let _ = dump_db(&db_guard, "dump.rdb");
    });
    RespFrame::SimpleString("Background saving started".into())
}
