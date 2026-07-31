use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_connection(mut stream: TcpStream) {
    let mut buf = [0u8; 512];
    loop {
        let bytes_read = match stream.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => n,
            Err(_) => return,
        };
        if stream.write_all(&buf[..bytes_read]).is_err() {
            return;
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
