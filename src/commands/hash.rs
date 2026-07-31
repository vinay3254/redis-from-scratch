use crate::db::Db;
use crate::resp::RespFrame;

const WRONG_TYPE_ERR: &str =
    "WRONGTYPE Operation against a key holding the wrong kind of value";

pub fn hset(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return RespFrame::Error("ERR wrong number of arguments for 'hset' command".into());
    }
    let key = &args[0];
    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for i in (1..args.len()).step_by(2) {
        pairs.push((args[i].clone(), args[i + 1].clone()));
    }

    match db.hset(key, &pairs) {
        Ok(count) => RespFrame::Integer(count as i64),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn hget(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'hget' command".into());
    }
    match db.hget(&args[0], &args[1]) {
        Ok(Some(val)) => RespFrame::BulkString(Some(val)),
        Ok(None) => RespFrame::BulkString(None),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn hgetall(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 1 {
        return RespFrame::Error("ERR wrong number of arguments for 'hgetall' command".into());
    }
    match db.hgetall(&args[0]) {
        Ok(pairs) => {
            let mut frames = Vec::with_capacity(pairs.len() * 2);
            for (field, val) in pairs {
                frames.push(RespFrame::BulkString(Some(field)));
                frames.push(RespFrame::BulkString(Some(val)));
            }
            RespFrame::Array(Some(frames))
        }
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn hdel(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'hdel' command".into());
    }
    match db.hdel(&args[0], &args[1..]) {
        Ok(count) => RespFrame::Integer(count as i64),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
