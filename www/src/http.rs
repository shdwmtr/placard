use std::io::{self, BufRead, Write};
use std::time::Instant;

#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
}

impl Request {
    pub fn query_param(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug)]
pub enum RequestError {
    TooLarge,
    ConnectionClosed,
    Malformed,
    TimedOut,
}

enum LineError {
    TooLong,
    Eof,
    Io,
    InvalidUtf8,
    TimedOut,
}

impl From<LineError> for RequestError {
    fn from(e: LineError) -> Self {
        match e {
            LineError::TooLong => RequestError::TooLarge,
            LineError::Eof => RequestError::ConnectionClosed,
            LineError::Io | LineError::InvalidUtf8 => RequestError::Malformed,
            LineError::TimedOut => RequestError::TimedOut,
        }
    }
}

fn read_line_capped(
    reader: &mut impl BufRead,
    max_len: usize,
    deadline: Instant,
) -> Result<String, LineError> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if Instant::now() >= deadline {
            return Err(LineError::TimedOut);
        }
        let n = reader.read(&mut byte).map_err(|_| LineError::Io)?;
        if n == 0 {
            return Err(LineError::Eof);
        }
        if byte[0] == b'\n' {
            break;
        }
        line.push(byte[0]);
        if line.len() > max_len {
            return Err(LineError::TooLong);
        }
    }
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    String::from_utf8(line).map_err(|_| LineError::InvalidUtf8)
}

fn parse_query(qs: &str) -> Vec<(String, String)> {
    if qs.is_empty() {
        return Vec::new();
    }
    qs.split('&')
        .filter_map(|pair| {
            pair.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect()
}

pub fn read_request(
    reader: &mut impl BufRead,
    max_total: usize,
    deadline: Instant,
) -> Result<Request, RequestError> {
    let mut remaining = max_total;

    let request_line = read_line_capped(reader, remaining, deadline)?;
    remaining = remaining.saturating_sub(request_line.len());

    let mut parts = request_line.split(' ');
    let method = parts.next().ok_or(RequestError::Malformed)?.to_string();
    let target = parts.next().ok_or(RequestError::Malformed)?.to_string();
    parts.next().ok_or(RequestError::Malformed)?;

    let (path, query_string) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target, String::new()),
    };
    let query = parse_query(&query_string);

    loop {
        let line = read_line_capped(reader, remaining, deadline)?;
        remaining = remaining.saturating_sub(line.len());
        if line.is_empty() {
            break;
        }
    }

    Ok(Request {
        method,
        path,
        query,
    })
}

pub fn write_response(
    stream: &mut impl Write,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    write_response_with_headers(stream, status, reason, content_type, body, &[])
}

pub fn write_response_with_headers(
    stream: &mut impl Write,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
    extra_headers: &[(&str, &str)],
) -> io::Result<()> {
    write!(stream, "HTTP/1.1 {status} {reason}\r\n")?;
    write!(stream, "Content-Type: {content_type}\r\n")?;
    write!(stream, "Content-Length: {}\r\n", body.len())?;
    write!(stream, "Connection: close\r\n")?;
    for (name, value) in extra_headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    write!(stream, "\r\n")?;
    stream.write_all(body)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::Duration;

    fn far_future_deadline() -> Instant {
        Instant::now() + Duration::from_secs(60)
    }

    #[test]
    fn parses_well_formed_request() {
        let raw =
            "GET /r/abc.png?width=200 HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test\r\n\r\n";
        let mut cursor = Cursor::new(raw.as_bytes());
        let req = read_request(&mut cursor, 8192, far_future_deadline()).expect("should parse");
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/r/abc.png");
        assert_eq!(req.query_param("width"), Some("200"));
    }

    #[test]
    fn parses_request_with_no_query_string() {
        let raw = "GET / HTTP/1.1\r\n\r\n";
        let mut cursor = Cursor::new(raw.as_bytes());
        let req = read_request(&mut cursor, 8192, far_future_deadline()).expect("should parse");
        assert_eq!(req.path, "/");
        assert!(req.query.is_empty());
    }

    #[test]
    fn missing_blank_line_terminator_is_connection_closed() {
        let raw = "GET / HTTP/1.1\r\nHost: example.com\r\n";
        let mut cursor = Cursor::new(raw.as_bytes());
        let err = read_request(&mut cursor, 8192, far_future_deadline()).unwrap_err();
        assert!(matches!(err, RequestError::ConnectionClosed));
    }

    #[test]
    fn oversized_request_line_is_rejected() {
        let raw = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(100));
        let mut cursor = Cursor::new(raw.as_bytes());
        let err = read_request(&mut cursor, 32, far_future_deadline()).unwrap_err();
        assert!(matches!(err, RequestError::TooLarge));
    }

    #[test]
    fn malformed_request_line_is_rejected() {
        let raw = "GARBAGE\r\n\r\n";
        let mut cursor = Cursor::new(raw.as_bytes());
        let err = read_request(&mut cursor, 8192, far_future_deadline()).unwrap_err();
        assert!(matches!(err, RequestError::Malformed));
    }

    #[test]
    fn expired_deadline_is_rejected_even_with_more_data_available() {
        let raw = "GET / HTTP/1.1\r\n\r\n";
        let mut cursor = Cursor::new(raw.as_bytes());
        let already_expired = Instant::now() - Duration::from_secs(1);
        let err = read_request(&mut cursor, 8192, already_expired).unwrap_err();
        assert!(matches!(err, RequestError::TimedOut));
    }
}
