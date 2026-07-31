mod commands;
mod db;
mod persistence;
mod pubsub;
mod resp;
mod skiplist;

use db::Db;
use persistence::aof::Aof;
use pubsub::PubSub;
use resp::RespFrame;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn parse_cmd_parts(frame: &RespFrame) -> (String, Vec<Vec<u8>>) {
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

fn handle_connection(
    mut stream: TcpStream,
    db: Arc<Mutex<Db>>,
    pubsub: Arc<Mutex<PubSub>>,
    aof: Arc<Aof>,
) {
    let mut buffer = Vec::new();
    let mut read_buf = [0u8; 512];

    let client_id = {
        let mut ps = pubsub.lock().unwrap();
        ps.generate_client_id()
    };

    let (tx, rx) = std::sync::mpsc::channel::<RespFrame>();
    let mut is_subscribed = false;
    let mut in_transaction = false;
    let mut tx_queue: Vec<RespFrame> = Vec::new();

    stream.set_read_timeout(Some(Duration::from_millis(100))).ok();

    loop {
        if is_subscribed {
            while let Ok(msg) = rx.try_recv() {
                if stream.write_all(&msg.serialize()).is_err() {
                    let mut ps = pubsub.lock().unwrap();
                    ps.remove_client(client_id);
                    return;
                }
            }
        }

        match stream.read(&mut read_buf) {
            Ok(0) => {
                let mut ps = pubsub.lock().unwrap();
                ps.remove_client(client_id);
                return;
            }
            Ok(n) => {
                buffer.extend_from_slice(&read_buf[..n]);
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => {
                let mut ps = pubsub.lock().unwrap();
                ps.remove_client(client_id);
                return;
            }
        }

        while !buffer.is_empty() {
            match RespFrame::parse(&buffer) {
                Ok(Some((frame, consumed))) => {
                    let (cmd_name, cmd_args) = parse_cmd_parts(&frame);
                    if cmd_name == "MULTI" {
                        if in_transaction {
                            let err = RespFrame::Error("ERR MULTI calls can not be nested".into());
                            let _ = stream.write_all(&err.serialize());
                        } else {
                            in_transaction = true;
                            tx_queue.clear();
                            let ok = RespFrame::SimpleString("OK".into());
                            let _ = stream.write_all(&ok.serialize());
                        }
                    } else if cmd_name == "DISCARD" {
                        if !in_transaction {
                            let err = RespFrame::Error("ERR DISCARD without MULTI".into());
                            let _ = stream.write_all(&err.serialize());
                        } else {
                            in_transaction = false;
                            tx_queue.clear();
                            let ok = RespFrame::SimpleString("OK".into());
                            let _ = stream.write_all(&ok.serialize());
                        }
                    } else if cmd_name == "EXEC" {
                        if !in_transaction {
                            let err = RespFrame::Error("ERR EXEC without MULTI".into());
                            let _ = stream.write_all(&err.serialize());
                        } else {
                            in_transaction = false;
                            let queue = std::mem::take(&mut tx_queue);
                            let response_frame = commands::tx::exec(
                                queue,
                                Arc::clone(&db),
                                Some(Arc::clone(&pubsub)),
                                Some(&aof),
                            );
                            if stream.write_all(&response_frame.serialize()).is_err() {
                                let mut ps = pubsub.lock().unwrap();
                                ps.remove_client(client_id);
                                return;
                            }
                        }
                    } else if in_transaction {
                        tx_queue.push(frame);
                        let queued = RespFrame::SimpleString("QUEUED".into());
                        if stream.write_all(&queued.serialize()).is_err() {
                            let mut ps = pubsub.lock().unwrap();
                            ps.remove_client(client_id);
                            return;
                        }
                    } else if cmd_name == "SUBSCRIBE" {
                        is_subscribed = true;
                        let responses = {
                            let mut ps = pubsub.lock().unwrap();
                            ps.subscribe(client_id, &cmd_args, tx.clone())
                        };
                        for resp in responses {
                            if stream.write_all(&resp.serialize()).is_err() {
                                return;
                            }
                        }
                    } else if cmd_name == "UNSUBSCRIBE" {
                        let responses = {
                            let mut ps = pubsub.lock().unwrap();
                            ps.unsubscribe(client_id, &cmd_args)
                        };
                        for resp in responses {
                            if stream.write_all(&resp.serialize()).is_err() {
                                return;
                            }
                        }
                    } else {
                        let response_frame = commands::dispatch(
                            frame,
                            Arc::clone(&db),
                            Some(Arc::clone(&pubsub)),
                            Some(&aof),
                        );
                        if stream.write_all(&response_frame.serialize()).is_err() {
                            let mut ps = pubsub.lock().unwrap();
                            ps.remove_client(client_id);
                            return;
                        }
                    }
                    buffer.drain(..consumed);
                }
                Ok(None) => break,
                Err(_) => {
                    let err_frame = RespFrame::Error("ERR protocol error".into());
                    let _ = stream.write_all(&err_frame.serialize());
                    let mut ps = pubsub.lock().unwrap();
                    ps.remove_client(client_id);
                    return;
                }
            }
        }
    }
}

fn main() {
    let mut db_instance = if Path::new("dump.rdb").exists() {
        match persistence::rdb::load_db("dump.rdb") {
            Ok(loaded_db) => loaded_db,
            Err(_) => Db::new(),
        }
    } else {
        Db::new()
    };

    if Path::new("appendonly.aof").exists() {
        Aof::replay("appendonly.aof", &mut db_instance).ok();
    }

    let db = Arc::new(Mutex::new(db_instance));
    let pubsub = Arc::new(Mutex::new(PubSub::new()));
    let aof = Arc::new(Aof::open("appendonly.aof").expect("failed to open appendonly.aof"));

    let db_active_expire = Arc::clone(&db);
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(100));
        let mut db_guard = db_active_expire.lock().unwrap();
        db_guard.purge_expired();
    });

    let listener = TcpListener::bind("127.0.0.1:6380").expect("failed to bind to port 6380");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let db_clone = Arc::clone(&db);
                let pubsub_clone = Arc::clone(&pubsub);
                let aof_clone = Arc::clone(&aof);
                thread::spawn(move || handle_connection(stream, db_clone, pubsub_clone, aof_clone));
            }
            Err(_) => continue,
        }
    }
}
