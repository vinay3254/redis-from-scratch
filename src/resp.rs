use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum RespFrame {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<RespFrame>>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ParseError {
    InvalidFrame(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidFrame(msg) => write!(f, "Invalid frame: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

impl RespFrame {
    pub fn parse(buf: &[u8]) -> Result<Option<(RespFrame, usize)>, ParseError> {
        if buf.is_empty() {
            return Ok(None);
        }

        match buf[0] {
            b'+' => parse_simple_string(buf),
            b'-' => parse_error(buf),
            b':' => parse_integer(buf),
            b'$' => parse_bulk_string(buf),
            b'*' => parse_array(buf),
            _ => Err(ParseError::InvalidFrame(format!(
                "Unknown frame type byte: {}",
                buf[0] as char
            ))),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode(&mut out);
        out
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            RespFrame::SimpleString(s) => {
                out.push(b'+');
                out.extend_from_slice(s.as_bytes());
                out.extend_from_slice(b"\r\n");
            }
            RespFrame::Error(s) => {
                out.push(b'-');
                out.extend_from_slice(s.as_bytes());
                out.extend_from_slice(b"\r\n");
            }
            RespFrame::Integer(n) => {
                out.push(b':');
                out.extend_from_slice(n.to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
            }
            RespFrame::BulkString(None) => {
                out.extend_from_slice(b"$-1\r\n");
            }
            RespFrame::BulkString(Some(data)) => {
                out.push(b'$');
                out.extend_from_slice(data.len().to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
                out.extend_from_slice(data);
                out.extend_from_slice(b"\r\n");
            }
            RespFrame::Array(None) => {
                out.extend_from_slice(b"*-1\r\n");
            }
            RespFrame::Array(Some(elements)) => {
                out.push(b'*');
                out.extend_from_slice(elements.len().to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
                for elem in elements {
                    elem.encode(out);
                }
            }
        }
    }
}

fn read_line(buf: &[u8]) -> Result<Option<(&[u8], usize)>, ParseError> {
    for i in 0..buf.len().saturating_sub(1) {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            return Ok(Some((&buf[..i], i + 2)));
        }
    }
    Ok(None)
}

fn parse_simple_string(buf: &[u8]) -> Result<Option<(RespFrame, usize)>, ParseError> {
    match read_line(&buf[1..]) {
        Ok(Some((line, consumed))) => {
            let s = String::from_utf8(line.to_vec())
                .map_err(|_| ParseError::InvalidFrame("Invalid UTF-8 in simple string".into()))?;
            Ok(Some((RespFrame::SimpleString(s), consumed + 1)))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

fn parse_error(buf: &[u8]) -> Result<Option<(RespFrame, usize)>, ParseError> {
    match read_line(&buf[1..]) {
        Ok(Some((line, consumed))) => {
            let s = String::from_utf8(line.to_vec())
                .map_err(|_| ParseError::InvalidFrame("Invalid UTF-8 in error string".into()))?;
            Ok(Some((RespFrame::Error(s), consumed + 1)))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

fn parse_integer(buf: &[u8]) -> Result<Option<(RespFrame, usize)>, ParseError> {
    match read_line(&buf[1..]) {
        Ok(Some((line, consumed))) => {
            let s = std::str::from_utf8(line)
                .map_err(|_| ParseError::InvalidFrame("Invalid UTF-8 in integer".into()))?;
            let n: i64 = s
                .parse()
                .map_err(|_| ParseError::InvalidFrame("Invalid integer format".into()))?;
            Ok(Some((RespFrame::Integer(n), consumed + 1)))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

fn parse_bulk_string(buf: &[u8]) -> Result<Option<(RespFrame, usize)>, ParseError> {
    match read_line(&buf[1..]) {
        Ok(Some((line, consumed_header))) => {
            let s = std::str::from_utf8(line)
                .map_err(|_| ParseError::InvalidFrame("Invalid UTF-8 in bulk string length".into()))?;
            let len: i64 = s
                .parse()
                .map_err(|_| ParseError::InvalidFrame("Invalid bulk string length".into()))?;

            if len < -1 {
                return Err(ParseError::InvalidFrame(
                    "Bulk string length cannot be less than -1".into(),
                ));
            }

            if len == -1 {
                return Ok(Some((RespFrame::BulkString(None), consumed_header + 1)));
            }

            let ulen = len as usize;
            let total_header = 1 + consumed_header;
            let remaining = &buf[total_header..];

            if remaining.len() < ulen + 2 {
                return Ok(None);
            }

            if remaining[ulen] != b'\r' || remaining[ulen + 1] != b'\n' {
                return Err(ParseError::InvalidFrame(
                    "Bulk string data must end with CRLF".into(),
                ));
            }

            let data = remaining[..ulen].to_vec();
            Ok(Some((
                RespFrame::BulkString(Some(data)),
                total_header + ulen + 2,
            )))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

fn parse_array(buf: &[u8]) -> Result<Option<(RespFrame, usize)>, ParseError> {
    match read_line(&buf[1..]) {
        Ok(Some((line, consumed_header))) => {
            let s = std::str::from_utf8(line)
                .map_err(|_| ParseError::InvalidFrame("Invalid UTF-8 in array length".into()))?;
            let count: i64 = s
                .parse()
                .map_err(|_| ParseError::InvalidFrame("Invalid array length".into()))?;

            if count < -1 {
                return Err(ParseError::InvalidFrame(
                    "Array length cannot be less than -1".into(),
                ));
            }

            if count == -1 {
                return Ok(Some((RespFrame::Array(None), consumed_header + 1)));
            }

            let ucount = count as usize;
            let mut offset = 1 + consumed_header;
            let mut elements = Vec::with_capacity(ucount);

            for _ in 0..ucount {
                if offset >= buf.len() {
                    return Ok(None);
                }
                match RespFrame::parse(&buf[offset..])? {
                    Some((frame, consumed)) => {
                        elements.push(frame);
                        offset += consumed;
                    }
                    None => return Ok(None),
                }
            }

            Ok(Some((RespFrame::Array(Some(elements)), offset)))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_string() {
        let input = b"+OK\r\n";
        let res = RespFrame::parse(input).unwrap();
        assert_eq!(res, Some((RespFrame::SimpleString("OK".into()), 5)));
        assert_eq!(RespFrame::SimpleString("OK".into()).serialize(), input);
    }

    #[test]
    fn test_error() {
        let input = b"-ERR unknown command\r\n";
        let res = RespFrame::parse(input).unwrap();
        assert_eq!(
            res,
            Some((RespFrame::Error("ERR unknown command".into()), 22))
        );
        assert_eq!(
            RespFrame::Error("ERR unknown command".into()).serialize(),
            input
        );
    }

    #[test]
    fn test_integer() {
        let input = b":1000\r\n";
        let res = RespFrame::parse(input).unwrap();
        assert_eq!(res, Some((RespFrame::Integer(1000), 7)));
        assert_eq!(RespFrame::Integer(1000).serialize(), input);

        let neg_input = b":-42\r\n";
        let neg_res = RespFrame::parse(neg_input).unwrap();
        assert_eq!(neg_res, Some((RespFrame::Integer(-42), 6)));
        assert_eq!(RespFrame::Integer(-42).serialize(), neg_input);
    }

    #[test]
    fn test_bulk_string() {
        let input = b"$5\r\nhello\r\n";
        let res = RespFrame::parse(input).unwrap();
        assert_eq!(
            res,
            Some((RespFrame::BulkString(Some(b"hello".to_vec())), 11))
        );
        assert_eq!(
            RespFrame::BulkString(Some(b"hello".to_vec())).serialize(),
            input
        );

        let empty_input = b"$0\r\n\r\n";
        let empty_res = RespFrame::parse(empty_input).unwrap();
        assert_eq!(
            empty_res,
            Some((RespFrame::BulkString(Some(b"".to_vec())), 6))
        );
        assert_eq!(
            RespFrame::BulkString(Some(b"".to_vec())).serialize(),
            empty_input
        );

        let null_input = b"$-1\r\n";
        let null_res = RespFrame::parse(null_input).unwrap();
        assert_eq!(null_res, Some((RespFrame::BulkString(None), 5)));
        assert_eq!(RespFrame::BulkString(None).serialize(), null_input);
    }

    #[test]
    fn test_array() {
        let input = b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        let res = RespFrame::parse(input).unwrap();
        let expected = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"foo".to_vec())),
            RespFrame::BulkString(Some(b"bar".to_vec())),
        ]));
        assert_eq!(res, Some((expected.clone(), input.len())));
        assert_eq!(expected.serialize(), input);

        let null_arr = b"*-1\r\n";
        assert_eq!(
            RespFrame::parse(null_arr).unwrap(),
            Some((RespFrame::Array(None), 5))
        );
        assert_eq!(RespFrame::Array(None).serialize(), null_arr);
    }

    #[test]
    fn test_incomplete() {
        assert_eq!(RespFrame::parse(b"+OK").unwrap(), None);
        assert_eq!(RespFrame::parse(b"$5\r\nhel").unwrap(), None);
        assert_eq!(
            RespFrame::parse(b"*2\r\n$3\r\nfoo\r\n").unwrap(),
            None
        );
    }
}
