use std::fmt;

/// Conservative limits applied before JSON parsing. Attachment payloads are
/// represented in JSON in v1 and therefore count toward `max_frame_bytes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameLimits {
    pub max_header_bytes: usize,
    pub max_frame_bytes: usize,
}

impl Default for FrameLimits {
    fn default() -> Self {
        Self {
            max_header_bytes: 8 * 1024,
            max_frame_bytes: 4 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    HeaderTooLarge,
    InvalidHeader,
    MissingContentLength,
    DuplicateContentLength,
    InvalidContentLength,
    FrameTooLarge { length: usize, limit: usize },
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderTooLarge => f.write_str("header exceeds configured limit"),
            Self::InvalidHeader => f.write_str("malformed frame header"),
            Self::MissingContentLength => f.write_str("missing Content-Length header"),
            Self::DuplicateContentLength => f.write_str("duplicate Content-Length header"),
            Self::InvalidContentLength => f.write_str("invalid Content-Length header"),
            Self::FrameTooLarge { length, limit } => {
                write!(f, "frame length {length} exceeds configured limit {limit}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// Incremental LSP-style frame decoder. A call can yield zero, one, or many
/// complete JSON byte bodies; partial headers and bodies stay buffered.
#[derive(Debug)]
pub struct FrameDecoder {
    limits: FrameLimits,
    buffer: Vec<u8>,
}

impl FrameDecoder {
    pub fn new(limits: FrameLimits) -> Self {
        Self {
            limits,
            buffer: Vec::new(),
        }
    }

    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, FrameError> {
        self.buffer.extend_from_slice(bytes);
        let mut frames = Vec::new();

        loop {
            let Some(header_end) = find_header_end(&self.buffer) else {
                if self.buffer.len() > self.limits.max_header_bytes {
                    return Err(FrameError::HeaderTooLarge);
                }
                break;
            };
            if header_end > self.limits.max_header_bytes {
                return Err(FrameError::HeaderTooLarge);
            }
            let length = parse_content_length(&self.buffer[..header_end])?;
            if length > self.limits.max_frame_bytes {
                return Err(FrameError::FrameTooLarge {
                    length,
                    limit: self.limits.max_frame_bytes,
                });
            }
            let body_start = header_end + 4;
            let Some(total) = body_start.checked_add(length) else {
                return Err(FrameError::InvalidContentLength);
            };
            if self.buffer.len() < total {
                break;
            }
            frames.push(self.buffer[body_start..total].to_vec());
            self.buffer.drain(..total);
        }
        Ok(frames)
    }

    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
    }
}

pub fn encode_frame(body: &[u8]) -> Vec<u8> {
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body);
    frame
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_content_length(header: &[u8]) -> Result<usize, FrameError> {
    let text = std::str::from_utf8(header).map_err(|_| FrameError::InvalidHeader)?;
    let mut value = None;
    for line in text.split("\r\n") {
        let Some((name, raw_value)) = line.split_once(':') else {
            return Err(FrameError::InvalidHeader);
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            if value.is_some() {
                return Err(FrameError::DuplicateContentLength);
            }
            let parsed = raw_value
                .trim()
                .parse::<usize>()
                .map_err(|_| FrameError::InvalidContentLength)?;
            value = Some(parsed);
        }
    }
    value.ok_or(FrameError::MissingContentLength)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_partial_and_multiple_frames() {
        let first = encode_frame(br#"{"one":1}"#);
        let second = encode_frame(br#"{"two":2}"#);
        let mut decoder = FrameDecoder::new(FrameLimits::default());
        assert!(decoder.push(&first[..7]).unwrap().is_empty());
        let mut remaining = first[7..].to_vec();
        remaining.extend_from_slice(&second);
        assert_eq!(
            decoder.push(&remaining).unwrap(),
            vec![br#"{"one":1}"#, br#"{"two":2}"#]
        );
    }

    #[test]
    fn rejects_malformed_and_oversized_frames() {
        let mut decoder = FrameDecoder::new(FrameLimits {
            max_header_bytes: 32,
            max_frame_bytes: 3,
        });
        assert_eq!(
            decoder.push(b"Length: 2\r\n\r\n{}"),
            Err(FrameError::MissingContentLength)
        );
        let mut decoder = FrameDecoder::new(FrameLimits {
            max_header_bytes: 32,
            max_frame_bytes: 3,
        });
        assert!(matches!(
            decoder.push(b"Content-Length: 4\r\n\r\n1234"),
            Err(FrameError::FrameTooLarge { .. })
        ));
    }
}
