mod resp;

use resp::RespFrame;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_connection(mut stream: TcpStream) {
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
                    let response = frame.serialize();
                    if stream.write_all(&response).is_err() {
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
    let listener = TcpListener::bind("127.0.0.1:6380").expect("failed to bind to port 6380");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || handle_connection(stream));
            }
            Err(_) => continue,
        }
    }
}
