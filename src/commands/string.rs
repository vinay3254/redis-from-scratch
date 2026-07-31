use crate::db::{Db, Value};
use crate::resp::RespFrame;

pub fn ping(args: &[Vec<u8>]) -> RespFrame {
    match args.len() {
        0 => RespFrame::SimpleString("PONG".into()),
        1 => RespFrame::BulkString(Some(args[0].clone())),
        _ => RespFrame::Error("ERR wrong number of arguments for 'ping' command".into()),
    }
}

pub fn echo(args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 1 {
        return RespFrame::Error("ERR wrong number of arguments for 'echo' command".into());
    }
    RespFrame::BulkString(Some(args[0].clone()))
}

pub fn set(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'set' command".into());
    }
    db.set(args[0].clone(), Value::String(args[1].clone()));
    RespFrame::SimpleString("OK".into())
}

pub fn get(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 1 {
        return RespFrame::Error("ERR wrong number of arguments for 'get' command".into());
    }
    match db.get(&args[0]) {
        Some(Value::String(val)) => RespFrame::BulkString(Some(val.clone())),
        Some(_) => RespFrame::Error(
            "WRONGTYPE Operation against a key holding the wrong kind of value".into(),
        ),
        None => RespFrame::BulkString(None),
    }
}
