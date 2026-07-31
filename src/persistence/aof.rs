use crate::db::Db;
use crate::resp::RespFrame;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::sync::{Arc, Mutex};

pub struct Aof {
    file: Arc<Mutex<File>>,
}

impl Aof {
    pub fn open(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)?;
        Ok(Aof {
            file: Arc::new(Mutex::new(file)),
        })
    }

    pub fn append(&self, frame: &RespFrame) -> std::io::Result<()> {
        let bytes = frame.serialize();
        let mut f = self.file.lock().unwrap();
        f.write_all(&bytes)?;
        f.flush()
    }

    pub fn replay(path: &str, db: &mut Db) -> std::io::Result<()> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        let mut read_buf = [0u8; 512];

        loop {
            let bytes_read = reader.read(&mut read_buf)?;
            if bytes_read == 0 {
                break;
            }
            buffer.extend_from_slice(&read_buf[..bytes_read]);

            loop {
                match RespFrame::parse(&buffer) {
                    Ok(Some((frame, consumed))) => {
                        crate::commands::dispatch_mutating(frame, db);
                        buffer.drain(..consumed);
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aof_append_and_replay() {
        let temp_path = "test_appendonly.aof";
        std::fs::remove_file(temp_path).ok();

        let aof = Aof::open(temp_path).unwrap();
        let set_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"SET".to_vec())),
            RespFrame::BulkString(Some(b"k1".to_vec())),
            RespFrame::BulkString(Some(b"v1".to_vec())),
        ]));
        aof.append(&set_frame).unwrap();

        let mut db = Db::new();
        Aof::replay(temp_path, &mut db).unwrap();
        assert_eq!(db.get(b"k1"), Some(&crate::db::Value::String(b"v1".to_vec())));

        std::fs::remove_file(temp_path).ok();
    }
}
