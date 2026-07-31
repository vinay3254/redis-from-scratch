use crate::db::Db;
use crate::resp::RespFrame;

const WRONG_TYPE_ERR: &str =
    "WRONGTYPE Operation against a key holding the wrong kind of value";

pub fn lpush(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'lpush' command".into());
    }
    match db.lpush(&args[0], &args[1..]) {
        Ok(len) => RespFrame::Integer(len as i64),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn rpush(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'rpush' command".into());
    }
    match db.rpush(&args[0], &args[1..]) {
        Ok(len) => RespFrame::Integer(len as i64),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn lpop(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 1 {
        return RespFrame::Error("ERR wrong number of arguments for 'lpop' command".into());
    }
    match db.lpop(&args[0]) {
        Ok(Some(val)) => RespFrame::BulkString(Some(val)),
        Ok(None) => RespFrame::BulkString(None),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn rpop(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 1 {
        return RespFrame::Error("ERR wrong number of arguments for 'rpop' command".into());
    }
    match db.rpop(&args[0]) {
        Ok(Some(val)) => RespFrame::BulkString(Some(val)),
        Ok(None) => RespFrame::BulkString(None),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn lrange(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 3 {
        return RespFrame::Error("ERR wrong number of arguments for 'lrange' command".into());
    }
    let start: i64 = match std::str::from_utf8(&args[1]).ok().and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => return RespFrame::Error("ERR value is not an integer or out of range".into()),
    };
    let stop: i64 = match std::str::from_utf8(&args[2]).ok().and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => return RespFrame::Error("ERR value is not an integer or out of range".into()),
    };

    match db.lrange(&args[0], start, stop) {
        Ok(elements) => {
            let frames = elements
                .into_iter()
                .map(|e| RespFrame::BulkString(Some(e)))
                .collect();
            RespFrame::Array(Some(frames))
        }
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
