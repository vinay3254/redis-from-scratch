use crate::db::Db;
use crate::persistence::aof::Aof;
use crate::pubsub::PubSub;
use crate::resp::RespFrame;
use std::sync::{Arc, Mutex};

pub fn exec(
    tx_queue: Vec<RespFrame>,
    db: Arc<Mutex<Db>>,
    pubsub: Option<Arc<Mutex<PubSub>>>,
    aof: Option<&Aof>,
) -> RespFrame {
    let mut results = Vec::with_capacity(tx_queue.len());
    let mut db_guard = db.lock().unwrap();

    for frame in tx_queue {
        let (cmd_name, _) = parse_cmd_name(&frame);

        if cmd_name == "PUBLISH" {
            if let Some(ref ps) = pubsub {
                if let RespFrame::Array(Some(ref elements)) = frame {
                    let mut args = Vec::new();
                    for elem in elements {
                        if let RespFrame::BulkString(Some(bytes)) = elem {
                            args.push(bytes.clone());
                        }
                    }
                    if args.len() >= 3 {
                        let mut ps_guard = ps.lock().unwrap();
                        let res = super::pubsub::publish(&mut ps_guard, &args[1..]);
                        results.push(res);
                        continue;
                    }
                }
            }
        }

        let result = super::dispatch_mutating(frame.clone(), &mut db_guard);

        if let RespFrame::Error(_) = &result {
        } else if super::is_write_command(&cmd_name) {
            if let Some(a) = aof {
                a.append(&frame).ok();
            }
        }

        results.push(result);
    }

    RespFrame::Array(Some(results))
}

fn parse_cmd_name(frame: &RespFrame) -> (String, Vec<Vec<u8>>) {
    let elements = match frame {
        RespFrame::Array(Some(elements)) if !elements.is_empty() => elements,
        _ => return (String::new(), Vec::new()),
    };

    let mut args = Vec::with_capacity(elements.len());
    for elem in elements {
        match elem {
            RespFrame::BulkString(Some(bytes)) => args.push(bytes.clone()),
            RespFrame::SimpleString(s) => args.push(s.clone().into_bytes()),
            _ => return (String::new(), Vec::new()),
        }
    }

    let cmd_name = match String::from_utf8(args[0].clone()) {
        Ok(s) => s.to_uppercase(),
        Err(_) => String::new(),
    };

    (cmd_name, args[1..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_atomic() {
        let db = Arc::new(Mutex::new(Db::new()));

        let queue = vec![
            RespFrame::Array(Some(vec![
                RespFrame::BulkString(Some(b"SET".to_vec())),
                RespFrame::BulkString(Some(b"k1".to_vec())),
                RespFrame::BulkString(Some(b"v1".to_vec())),
            ])),
            RespFrame::Array(Some(vec![
                RespFrame::BulkString(Some(b"GET".to_vec())),
                RespFrame::BulkString(Some(b"k1".to_vec())),
            ])),
        ];

        let res = exec(queue, Arc::clone(&db), None, None);

        let expected = RespFrame::Array(Some(vec![
            RespFrame::SimpleString("OK".into()),
            RespFrame::BulkString(Some(b"v1".to_vec())),
        ]));

        assert_eq!(res, expected);
    }
}
