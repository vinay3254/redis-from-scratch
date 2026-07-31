use crate::db::Db;
use crate::resp::RespFrame;

const WRONG_TYPE_ERR: &str =
    "WRONGTYPE Operation against a key holding the wrong kind of value";

pub fn zadd(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return RespFrame::Error("ERR wrong number of arguments for 'zadd' command".into());
    }
    let key = &args[0];
    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for i in (1..args.len()).step_by(2) {
        let score: f64 = match std::str::from_utf8(&args[i]).ok().and_then(|s| s.parse().ok()) {
            Some(s) => s,
            None => return RespFrame::Error("ERR value is not a valid float".into()),
        };
        pairs.push((score, args[i + 1].clone()));
    }

    match db.zadd(key, &pairs) {
        Ok(count) => RespFrame::Integer(count as i64),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn zscore(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'zscore' command".into());
    }
    match db.zscore(&args[0], &args[1]) {
        Ok(Some(score)) => RespFrame::BulkString(Some(score.to_string().into_bytes())),
        Ok(None) => RespFrame::BulkString(None),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}

pub fn zrange(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 3 || args.len() > 4 {
        return RespFrame::Error("ERR wrong number of arguments for 'zrange' command".into());
    }
    let start: i64 = match std::str::from_utf8(&args[1]).ok().and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => return RespFrame::Error("ERR value is not an integer or out of range".into()),
    };
    let stop: i64 = match std::str::from_utf8(&args[2]).ok().and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => return RespFrame::Error("ERR value is not an integer or out of range".into()),
    };

    let with_scores = if args.len() == 4 {
        match std::str::from_utf8(&args[3]) {
            Ok(s) if s.to_uppercase() == "WITHSCORES" => true,
            _ => return RespFrame::Error("ERR syntax error".into()),
        }
    } else {
        false
    };

    match db.zrange(&args[0], start, stop) {
        Ok(pairs) => {
            let mut frames = Vec::new();
            for (member, score) in pairs {
                frames.push(RespFrame::BulkString(Some(member)));
                if with_scores {
                    frames.push(RespFrame::BulkString(Some(score.to_string().into_bytes())));
                }
            }
            RespFrame::Array(Some(frames))
        }
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
