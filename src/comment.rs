use crate::{error::Error, name::Name};
use std::{borrow::Cow, collections::BTreeSet, ops::Range};

#[derive(serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Extension {
    None,
    Tc,
    Unknown,
}

#[derive(serde::Serialize)]
pub struct Record<'a> {
    pub name: &'a str,
    pub comment: Cow<'a, str>,
    pub extension: Extension,
    #[serde(skip)]
    pub range: Range<usize>,
    #[serde(skip)]
    pub line: usize,
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Record<'_>>, Error> {
    records(bytes)?.collect()
}

/// Keeps valid records and reports each bad physical line independently.
pub fn parse_for_list(bytes: &[u8]) -> Result<(Vec<Record<'_>>, Vec<Error>), Error> {
    let mut valid = Vec::new();
    let mut errors = Vec::new();
    for record in records(bytes)? {
        match record {
            Ok(record) => valid.push(record),
            Err(error) => errors.push(error),
        }
    }
    Ok((valid, errors))
}

fn records(bytes: &[u8]) -> Result<impl Iterator<Item = Result<Record<'_>, Error>>, Error> {
    let body = bytes
        .strip_prefix(b"\xef\xbb\xbf")
        .ok_or_else(|| Error::new("invalid_encoding", "Missing UTF-8 BOM", 1))?;
    let mut names = BTreeSet::new();
    let mut remaining = body;
    let mut line_number = 0;
    Ok(std::iter::from_fn(move || {
        loop {
            if remaining.is_empty() {
                return None;
            }
            line_number += 1;
            let end = remaining
                .iter()
                .position(|byte| matches!(byte, b'\r' | b'\n'))
                .unwrap_or(remaining.len());
            let terminator = if remaining[end..].starts_with(b"\r\n") {
                2
            } else if end < remaining.len() {
                1
            } else {
                0
            };
            let start = bytes.len() - remaining.len();
            let line = &remaining[..end];
            remaining = &remaining[end + terminator..];
            if line.is_empty() {
                continue;
            }
            return Some(parse_record(
                line,
                start..start + end + terminator,
                line_number,
                &mut names,
            ));
        }
    }))
}

fn parse_record<'a>(
    bytes: &'a [u8],
    range: Range<usize>,
    line_number: usize,
    names: &mut BTreeSet<Name>,
) -> Result<Record<'a>, Error> {
    let line = std::str::from_utf8(bytes)
        .map_err(|_| at_line("invalid_encoding", "Record is not valid UTF-8", line_number))?;
    if range.len() > 4096 {
        return Err(at_line(
            "invalid_format",
            "Physical record exceeds 4096 bytes",
            line_number,
        ));
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
    if !names.insert(Name::new(name)) {
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
    Ok(Record {
        name,
        comment,
        extension,
        range,
        line: line_number,
    })
}

pub fn set(original: Option<&[u8]>, name: &str, body: &str) -> Result<Option<Vec<u8>>, Error> {
    let bytes = original.unwrap_or(b"\xef\xbb\xbf\r\n");
    let records = parse(bytes)?;
    let key = Name::new(name);
    let target = records.iter().find(|record| key.matches(record.name));
    if let Some(record) = target {
        if matches!(record.extension, Extension::Unknown) {
            return Err(Error::new(
                "unknown_extension",
                "Unknown extension; cannot modify this record",
                1,
            ));
        }
        if record.comment == body {
            return Ok(None);
        }
    }
    let name = target.map_or(name, |record| record.name);
    let body = if body.contains('\n') {
        format!(
            "{}\u{4}\u{c2}",
            body.replace('\\', "\\\\").replace('\n', "\\n")
        )
    } else {
        body.to_owned()
    };
    let record = if name.contains(' ') {
        format!("\"{name}\" {body}\r\n")
    } else {
        format!("{name} {body}\r\n")
    };
    if record.len() > 4096 {
        let mut error = Error::new("invalid_format", "Serialized record exceeds 4096 bytes", 1);
        error.line = target.map(|record| record.line);
        return Err(error);
    }
    let mut updated = Vec::with_capacity(bytes.len() + record.len());
    if let Some(target) = target {
        updated.extend_from_slice(&bytes[..target.range.start]);
        updated.extend_from_slice(record.as_bytes());
        updated.extend_from_slice(&bytes[target.range.end..]);
    } else {
        updated.extend_from_slice(bytes);
        if bytes.len() > 3 && !bytes.ends_with(b"\r") && !bytes.ends_with(b"\n") {
            if let Some(record) = records
                .last()
                .filter(|record| record.range.len() + 2 > 4096)
            {
                return Err(at_line(
                    "invalid_format",
                    "Final record exceeds 4096 bytes after adding CRLF",
                    record.line,
                ));
            }
            updated.extend_from_slice(b"\r\n");
        }
        updated.extend_from_slice(record.as_bytes());
    }
    Ok(Some(updated))
}

pub enum Removal {
    Unchanged,
    Update(Vec<u8>),
    DeleteFile,
}

pub fn remove(bytes: &[u8], name: &str) -> Result<Removal, Error> {
    let records = parse(bytes)?;
    let key = Name::new(name);
    let Some(target) = records.iter().find(|record| key.matches(record.name)) else {
        return Ok(Removal::Unchanged);
    };
    if records.len() == 1 {
        return Ok(Removal::DeleteFile);
    }
    let mut updated = Vec::with_capacity(bytes.len() - target.range.len());
    updated.extend_from_slice(&bytes[..target.range.start]);
    updated.extend_from_slice(&bytes[target.range.end..]);
    Ok(Removal::Update(updated))
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
