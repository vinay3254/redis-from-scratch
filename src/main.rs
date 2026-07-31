mod commands;
mod db;
mod persistence;
mod resp;
mod skiplist;

use db::Db;
use resp::RespFrame;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn handle_connection(mut stream: TcpStream, db: Arc<Mutex<Db>>) {
    let mut buffer = Vec::new();
    let mut read_buf = [0u8; 512];

    loop {
        let bytes_read = match stream.read(&mut read_buf) {
            Ok(0) => return,
            Ok(n) => n,
            Err(_) => return,
        };

        buffer.extend_from_slice(&read_buf[..bytes_read]);

        loop {
            match RespFrame::parse(&buffer) {
                Ok(Some((frame, consumed))) => {
                    let response_frame = commands::dispatch(frame, Arc::clone(&db));
                    let response_bytes = response_frame.serialize();
                    if stream.write_all(&response_bytes).is_err() {
                        return;
                    }
                    buffer.drain(..consumed);
                }
                Ok(None) => break,
                Err(_) => {
                    let err_frame = RespFrame::Error("ERR protocol error".into());
                    let _ = stream.write_all(&err_frame.serialize());
                    return;
                }
            }
        }
    }
}

fn main() {
    let db_instance = if Path::new("dump.rdb").exists() {
        match persistence::rdb::load_db("dump.rdb") {
            Ok(loaded_db) => loaded_db,
            Err(_) => Db::new(),
        }
    } else {
        Db::new()
    };

    let db = Arc::new(Mutex::new(db_instance));

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
                thread::spawn(move || handle_connection(stream, db_clone));
            }
            Err(_) => continue,
        }
    }
}
