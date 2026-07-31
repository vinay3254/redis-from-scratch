pub mod generic;
pub mod hash;
pub mod list;
pub mod set;
pub mod string;
pub mod zset;

use crate::db::Db;
use crate::resp::RespFrame;
use std::sync::{Arc, Mutex};

pub fn dispatch(frame: RespFrame, db: Arc<Mutex<Db>>) -> RespFrame {
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

    if cmd_name == "BGSAVE" {
        return generic::bgsave(db, cmd_args);
    }

    let mut db_guard = db.lock().unwrap();

    match cmd_name.as_str() {
        "PING" => string::ping(cmd_args),
        "ECHO" => string::echo(cmd_args),
        "SET" => string::set(&mut db_guard, cmd_args),
        "GET" => string::get(&mut db_guard, cmd_args),
        "DEL" => generic::del(&mut db_guard, cmd_args),
        "EXISTS" => generic::exists(&mut db_guard, cmd_args),
        "EXPIRE" => generic::expire(&mut db_guard, cmd_args),
        "PEXPIRE" => generic::pexpire(&mut db_guard, cmd_args),
        "TTL" => generic::ttl(&mut db_guard, cmd_args),
        "SAVE" => generic::save(&db_guard, cmd_args),
        "LPUSH" => list::lpush(&mut db_guard, cmd_args),
        "RPUSH" => list::rpush(&mut db_guard, cmd_args),
        "LPOP" => list::lpop(&mut db_guard, cmd_args),
        "RPOP" => list::rpop(&mut db_guard, cmd_args),
        "LRANGE" => list::lrange(&mut db_guard, cmd_args),
        "HSET" => hash::hset(&mut db_guard, cmd_args),
        "HGET" => hash::hget(&mut db_guard, cmd_args),
        "HGETALL" => hash::hgetall(&mut db_guard, cmd_args),
        "HDEL" => hash::hdel(&mut db_guard, cmd_args),
        "SADD" => set::sadd(&mut db_guard, cmd_args),
        "SREM" => set::srem(&mut db_guard, cmd_args),
        "SMEMBERS" => set::smembers(&mut db_guard, cmd_args),
        "SISMEMBER" => set::sismember(&mut db_guard, cmd_args),
        "ZADD" => zset::zadd(&mut db_guard, cmd_args),
        "ZSCORE" => zset::zscore(&mut db_guard, cmd_args),
        "ZRANGE" => zset::zrange(&mut db_guard, cmd_args),
        _ => RespFrame::Error(format!("ERR unknown command '{}'", cmd_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    fn new_db_arc() -> Arc<Mutex<Db>> {
        Arc::new(Mutex::new(Db::new()))
    }

    #[test]
    fn test_ping_echo() {
        let db = new_db_arc();

        let ping_frame = RespFrame::Array(Some(vec![RespFrame::BulkString(Some(
            b"PING".to_vec(),
        ))]));
        assert_eq!(
            dispatch(ping_frame, Arc::clone(&db)),
            RespFrame::SimpleString("PONG".into())
        );

        let echo_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"ECHO".to_vec())),
            RespFrame::BulkString(Some(b"hello".to_vec())),
        ]));
        assert_eq!(
            dispatch(echo_frame, Arc::clone(&db)),
            RespFrame::BulkString(Some(b"hello".to_vec()))
        );
    }

    #[test]
    fn test_set_get_del_exists() {
        let db = new_db_arc();

        let set_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"SET".to_vec())),
            RespFrame::BulkString(Some(b"key1".to_vec())),
            RespFrame::BulkString(Some(b"val1".to_vec())),
        ]));
        assert_eq!(
            dispatch(set_frame, Arc::clone(&db)),
            RespFrame::SimpleString("OK".into())
        );

        let get_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"GET".to_vec())),
            RespFrame::BulkString(Some(b"key1".to_vec())),
        ]));
        assert_eq!(
            dispatch(get_frame.clone(), Arc::clone(&db)),
            RespFrame::BulkString(Some(b"val1".to_vec()))
        );

        let exists_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"EXISTS".to_vec())),
            RespFrame::BulkString(Some(b"key1".to_vec())),
            RespFrame::BulkString(Some(b"key2".to_vec())),
        ]));
        assert_eq!(dispatch(exists_frame, Arc::clone(&db)), RespFrame::Integer(1));

        let del_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"DEL".to_vec())),
            RespFrame::BulkString(Some(b"key1".to_vec())),
        ]));
        assert_eq!(dispatch(del_frame, Arc::clone(&db)), RespFrame::Integer(1));

        assert_eq!(dispatch(get_frame, Arc::clone(&db)), RespFrame::BulkString(None));
    }

    #[test]
    fn test_expire_ttl() {
        let db = new_db_arc();

        let set_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"SET".to_vec())),
            RespFrame::BulkString(Some(b"k1".to_vec())),
            RespFrame::BulkString(Some(b"v1".to_vec())),
        ]));
        dispatch(set_frame, Arc::clone(&db));

        let pexpire_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"PEXPIRE".to_vec())),
            RespFrame::BulkString(Some(b"k1".to_vec())),
            RespFrame::BulkString(Some(b"50".to_vec())),
        ]));
        assert_eq!(dispatch(pexpire_frame, Arc::clone(&db)), RespFrame::Integer(1));

        let ttl_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"TTL".to_vec())),
            RespFrame::BulkString(Some(b"k1".to_vec())),
        ]));
        let res = dispatch(ttl_frame.clone(), Arc::clone(&db));
        assert!(matches!(res, RespFrame::Integer(_)));

        sleep(Duration::from_millis(60));

        assert_eq!(dispatch(ttl_frame, Arc::clone(&db)), RespFrame::Integer(-2));
    }

    #[test]
    fn test_list_commands() {
        let db = new_db_arc();

        let rpush_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"RPUSH".to_vec())),
            RespFrame::BulkString(Some(b"mylist".to_vec())),
            RespFrame::BulkString(Some(b"a".to_vec())),
            RespFrame::BulkString(Some(b"b".to_vec())),
        ]));
        assert_eq!(dispatch(rpush_frame, Arc::clone(&db)), RespFrame::Integer(2));

        let lpush_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"LPUSH".to_vec())),
            RespFrame::BulkString(Some(b"mylist".to_vec())),
            RespFrame::BulkString(Some(b"first".to_vec())),
        ]));
        assert_eq!(dispatch(lpush_frame, Arc::clone(&db)), RespFrame::Integer(3));

        let lrange_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"LRANGE".to_vec())),
            RespFrame::BulkString(Some(b"mylist".to_vec())),
            RespFrame::BulkString(Some(b"0".to_vec())),
            RespFrame::BulkString(Some(b"-1".to_vec())),
        ]));
        let expected_lrange = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"first".to_vec())),
            RespFrame::BulkString(Some(b"a".to_vec())),
            RespFrame::BulkString(Some(b"b".to_vec())),
        ]));
        assert_eq!(dispatch(lrange_frame, Arc::clone(&db)), expected_lrange);

        let lpop_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"LPOP".to_vec())),
            RespFrame::BulkString(Some(b"mylist".to_vec())),
        ]));
        assert_eq!(
            dispatch(lpop_frame, Arc::clone(&db)),
            RespFrame::BulkString(Some(b"first".to_vec()))
        );

        let rpop_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"RPOP".to_vec())),
            RespFrame::BulkString(Some(b"mylist".to_vec())),
        ]));
        assert_eq!(
            dispatch(rpop_frame, Arc::clone(&db)),
            RespFrame::BulkString(Some(b"b".to_vec()))
        );
    }

    #[test]
    fn test_hash_commands() {
        let db = new_db_arc();

        let hset_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"HSET".to_vec())),
            RespFrame::BulkString(Some(b"myhash".to_vec())),
            RespFrame::BulkString(Some(b"f1".to_vec())),
            RespFrame::BulkString(Some(b"v1".to_vec())),
            RespFrame::BulkString(Some(b"f2".to_vec())),
            RespFrame::BulkString(Some(b"v2".to_vec())),
        ]));
        assert_eq!(dispatch(hset_frame, Arc::clone(&db)), RespFrame::Integer(2));

        let hget_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"HGET".to_vec())),
            RespFrame::BulkString(Some(b"myhash".to_vec())),
            RespFrame::BulkString(Some(b"f1".to_vec())),
        ]));
        assert_eq!(
            dispatch(hget_frame, Arc::clone(&db)),
            RespFrame::BulkString(Some(b"v1".to_vec()))
        );

        let hdel_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"HDEL".to_vec())),
            RespFrame::BulkString(Some(b"myhash".to_vec())),
            RespFrame::BulkString(Some(b"f1".to_vec())),
        ]));
        assert_eq!(dispatch(hdel_frame, Arc::clone(&db)), RespFrame::Integer(1));

        let hgetall_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"HGETALL".to_vec())),
            RespFrame::BulkString(Some(b"myhash".to_vec())),
        ]));
        let res = dispatch(hgetall_frame, Arc::clone(&db));
        match res {
            RespFrame::Array(Some(arr)) => assert_eq!(arr.len(), 2),
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_set_commands() {
        let db = new_db_arc();

        let sadd_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"SADD".to_vec())),
            RespFrame::BulkString(Some(b"myset".to_vec())),
            RespFrame::BulkString(Some(b"m1".to_vec())),
            RespFrame::BulkString(Some(b"m2".to_vec())),
        ]));
        assert_eq!(dispatch(sadd_frame, Arc::clone(&db)), RespFrame::Integer(2));

        let sismember_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"SISMEMBER".to_vec())),
            RespFrame::BulkString(Some(b"myset".to_vec())),
            RespFrame::BulkString(Some(b"m1".to_vec())),
        ]));
        assert_eq!(dispatch(sismember_frame, Arc::clone(&db)), RespFrame::Integer(1));

        let smembers_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"SMEMBERS".to_vec())),
            RespFrame::BulkString(Some(b"myset".to_vec())),
        ]));
        match dispatch(smembers_frame, Arc::clone(&db)) {
            RespFrame::Array(Some(arr)) => assert_eq!(arr.len(), 2),
            _ => panic!("Expected array"),
        }

        let srem_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"SREM".to_vec())),
            RespFrame::BulkString(Some(b"myset".to_vec())),
            RespFrame::BulkString(Some(b"m1".to_vec())),
        ]));
        assert_eq!(dispatch(srem_frame, Arc::clone(&db)), RespFrame::Integer(1));
    }

    #[test]
    fn test_zset_commands() {
        let db = new_db_arc();

        let zadd_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"ZADD".to_vec())),
            RespFrame::BulkString(Some(b"myzset".to_vec())),
            RespFrame::BulkString(Some(b"10".to_vec())),
            RespFrame::BulkString(Some(b"one".to_vec())),
            RespFrame::BulkString(Some(b"20".to_vec())),
            RespFrame::BulkString(Some(b"two".to_vec())),
        ]));
        assert_eq!(dispatch(zadd_frame, Arc::clone(&db)), RespFrame::Integer(2));

        let zscore_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"ZSCORE".to_vec())),
            RespFrame::BulkString(Some(b"myzset".to_vec())),
            RespFrame::BulkString(Some(b"two".to_vec())),
        ]));
        assert_eq!(
            dispatch(zscore_frame, Arc::clone(&db)),
            RespFrame::BulkString(Some(b"20".to_vec()))
        );

        let zrange_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"ZRANGE".to_vec())),
            RespFrame::BulkString(Some(b"myzset".to_vec())),
            RespFrame::BulkString(Some(b"0".to_vec())),
            RespFrame::BulkString(Some(b"-1".to_vec())),
            RespFrame::BulkString(Some(b"WITHSCORES".to_vec())),
        ]));
        let expected_zrange = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"one".to_vec())),
            RespFrame::BulkString(Some(b"10".to_vec())),
            RespFrame::BulkString(Some(b"two".to_vec())),
            RespFrame::BulkString(Some(b"20".to_vec())),
        ]));
        assert_eq!(dispatch(zrange_frame, Arc::clone(&db)), expected_zrange);
    }
}
