pub mod generic;
pub mod string;

use crate::db::Db;
use crate::resp::RespFrame;

pub fn dispatch(frame: RespFrame, db: &mut Db) -> RespFrame {
    let elements = match frame {
        RespFrame::Array(Some(elements)) if !elements.is_empty() => elements,
        _ => return RespFrame::Error("ERR command must be a non-empty array".into()),
    };

    let mut args = Vec::with_capacity(elements.len());
    for elem in elements {
        match elem {
            RespFrame::BulkString(Some(bytes)) => args.push(bytes),
            RespFrame::SimpleString(s) => args.push(s.into_bytes()),
            _ => return RespFrame::Error("ERR command parts must be strings".into()),
        }
    }

    let cmd_name = match String::from_utf8(args[0].clone()) {
        Ok(s) => s.to_uppercase(),
        Err(_) => return RespFrame::Error("ERR invalid command name".into()),
    };

    let cmd_args = &args[1..];

    match cmd_name.as_str() {
        "PING" => string::ping(cmd_args),
        "ECHO" => string::echo(cmd_args),
        "SET" => string::set(db, cmd_args),
        "GET" => string::get(db, cmd_args),
        "DEL" => generic::del(db, cmd_args),
        "EXISTS" => generic::exists(db, cmd_args),
        "EXPIRE" => generic::expire(db, cmd_args),
        "PEXPIRE" => generic::pexpire(db, cmd_args),
        "TTL" => generic::ttl(db, cmd_args),
        _ => RespFrame::Error(format!("ERR unknown command '{}'", cmd_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_ping_echo() {
        let mut db = Db::new();

        let ping_frame = RespFrame::Array(Some(vec![RespFrame::BulkString(Some(
            b"PING".to_vec(),
        ))]));
        assert_eq!(
            dispatch(ping_frame, &mut db),
            RespFrame::SimpleString("PONG".into())
        );

        let echo_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"ECHO".to_vec())),
            RespFrame::BulkString(Some(b"hello".to_vec())),
        ]));
        assert_eq!(
            dispatch(echo_frame, &mut db),
            RespFrame::BulkString(Some(b"hello".to_vec()))
        );
    }

    #[test]
    fn test_set_get_del_exists() {
        let mut db = Db::new();

        let set_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"SET".to_vec())),
            RespFrame::BulkString(Some(b"key1".to_vec())),
            RespFrame::BulkString(Some(b"val1".to_vec())),
        ]));
        assert_eq!(
            dispatch(set_frame, &mut db),
            RespFrame::SimpleString("OK".into())
        );

        let get_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"GET".to_vec())),
            RespFrame::BulkString(Some(b"key1".to_vec())),
        ]));
        assert_eq!(
            dispatch(get_frame.clone(), &mut db),
            RespFrame::BulkString(Some(b"val1".to_vec()))
        );

        let exists_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"EXISTS".to_vec())),
            RespFrame::BulkString(Some(b"key1".to_vec())),
            RespFrame::BulkString(Some(b"key2".to_vec())),
        ]));
        assert_eq!(dispatch(exists_frame, &mut db), RespFrame::Integer(1));

        let del_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"DEL".to_vec())),
            RespFrame::BulkString(Some(b"key1".to_vec())),
        ]));
        assert_eq!(dispatch(del_frame, &mut db), RespFrame::Integer(1));

        assert_eq!(dispatch(get_frame, &mut db), RespFrame::BulkString(None));
    }

    #[test]
    fn test_expire_ttl() {
        let mut db = Db::new();

        let set_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"SET".to_vec())),
            RespFrame::BulkString(Some(b"k1".to_vec())),
            RespFrame::BulkString(Some(b"v1".to_vec())),
        ]));
        dispatch(set_frame, &mut db);

        let pexpire_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"PEXPIRE".to_vec())),
            RespFrame::BulkString(Some(b"k1".to_vec())),
            RespFrame::BulkString(Some(b"50".to_vec())),
        ]));
        assert_eq!(dispatch(pexpire_frame, &mut db), RespFrame::Integer(1));

        let ttl_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"TTL".to_vec())),
            RespFrame::BulkString(Some(b"k1".to_vec())),
        ]));
        let res = dispatch(ttl_frame.clone(), &mut db);
        assert!(matches!(res, RespFrame::Integer(_)));

        sleep(Duration::from_millis(60));

        assert_eq!(dispatch(ttl_frame, &mut db), RespFrame::Integer(-2));
    }
}
