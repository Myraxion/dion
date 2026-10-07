use crate::error::Error;
use std::{borrow::Cow, collections::HashSet};

#[derive(serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Extension {
    None,
    Tc,
    Unknown,
}

pub struct Record<'a> {
    pub name: &'a str,
    pub comment: Cow<'a, str>,
    pub extension: Extension,
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Record<'_>>, Error> {
    let body = bytes
        .strip_prefix(b"\xef\xbb\xbf")
        .ok_or_else(|| Error::new("invalid_encoding", "Missing UTF-8 BOM", 1))?;
    let mut records = Vec::new();
    let mut names = HashSet::new();
    let mut remaining = body;
    let mut line_number = 0;
    while !remaining.is_empty() {
        line_number += 1;
        let end = remaining
            .iter()
            .position(|byte| matches!(byte, b'\r' | b'\n'))
            .unwrap_or(remaining.len());
        let line = std::str::from_utf8(&remaining[..end])
            .map_err(|_| at_line("invalid_encoding", "Record is not valid UTF-8", line_number))?;
        let terminator = if remaining[end..].starts_with(b"\r\n") {
            2
        } else if end < remaining.len() {
            1
        } else {
            0
        };
        if end + terminator > 4096 {
            return Err(at_line(
                "invalid_format",
                "Physical record exceeds 4096 bytes",
                line_number,
            ));
        }
        remaining = &remaining[end + terminator..];
        if line.is_empty() {
            continue;
        }
        let (name, comment) = if let Some(quoted) = line.strip_prefix('"') {
            let close = quoted
                .find('"')
                .ok_or_else(|| at_line("invalid_format", "Unclosed name quote", line_number))?;
            let tail = &quoted[close + 1..];
            (
                &quoted[..close],
                if tail.is_empty() {
                    ""
                } else {
                    tail.strip_prefix(' ').ok_or_else(|| {
                        at_line(
                            "invalid_format",
                            "Expected a space after quoted name",
                            line_number,
                        )
                    })?
                },
            )
        } else {
            line.split_once(' ').unwrap_or((line, ""))
        };
        if name.is_empty() {
            return Err(at_line(
                "invalid_format",
                "Record name is empty",
                line_number,
            ));
        }
        if !names.insert(name.to_lowercase()) {
            return Err(at_line(
                "invalid_format",
                "Duplicate record name (case insensitive)",
                line_number,
            ));
        }
        let (comment, extension) = match comment.split_once('\u{4}') {
            Some((body, "\u{c2}")) => (Cow::Owned(decode_tc(body)), Extension::Tc),
            Some((body, _)) => (Cow::Borrowed(body), Extension::Unknown),
            None => (Cow::Borrowed(comment), Extension::None),
        };
        records.push(Record {
            name,
            comment,
            extension,
        });
    }
    Ok(records)
}

fn decode_tc(body: &str) -> String {
    let mut decoded = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some('n') => {
                    chars.next();
                    decoded.push('\n');
                    continue;
                }
                Some('\\') => {
                    chars.next();
                    decoded.push('\\');
                    continue;
                }
                _ => {}
            }
        }
        decoded.push(ch);
    }
    decoded
}

fn at_line(code: &'static str, message: &str, line: usize) -> Error {
    let mut error = Error::new(code, message, 1);
    error.line = Some(line);
    error
}
