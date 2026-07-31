use crate::db::Db;
use crate::resp::RespFrame;

const WRONG_TYPE_ERR: &str =
    "WRONGTYPE Operation against a key holding the wrong kind of value";

pub fn sadd(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'sadd' command".into());
    }
    match db.sadd(&args[0], &args[1..]) {
        Ok(count) => RespFrame::Integer(count as i64),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn srem(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'srem' command".into());
    }
    match db.srem(&args[0], &args[1..]) {
        Ok(count) => RespFrame::Integer(count as i64),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn smembers(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 1 {
        return RespFrame::Error("ERR wrong number of arguments for 'smembers' command".into());
    }
    match db.smembers(&args[0]) {
        Ok(members) => {
            let frames = members
                .into_iter()
                .map(|m| RespFrame::BulkString(Some(m)))
                .collect();
            RespFrame::Array(Some(frames))
        }
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn sismember(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'sismember' command".into());
    }
    match db.sismember(&args[0], &args[1]) {
        Ok(true) => RespFrame::Integer(1),
        Ok(false) => RespFrame::Integer(0),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
