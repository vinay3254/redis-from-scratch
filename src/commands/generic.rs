use crate::db::Db;
use crate::resp::RespFrame;

pub fn del(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.is_empty() {
        return RespFrame::Error("ERR wrong number of arguments for 'del' command".into());
    }
    let count = db.del(args);
    RespFrame::Integer(count as i64)
}

pub fn exists(db: &Db, args: &[Vec<u8>]) -> RespFrame {
    if args.is_empty() {
        return RespFrame::Error("ERR wrong number of arguments for 'exists' command".into());
    }
    let count = db.exists(args);
    RespFrame::Integer(count as i64)
}
