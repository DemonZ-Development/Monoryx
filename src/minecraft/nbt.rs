use std::{collections::BTreeMap, io::Read, path::Path};

#[derive(Debug, Clone)]
pub enum Value {
    Number(i64),
    Text(String),
    List(Vec<Value>),
    Compound(BTreeMap<String, Value>),
    Skipped,
}

impl Value {
    pub fn get(&self, name: &str) -> Option<&Self> {
        if let Self::Compound(values) = self {
            values.get(name)
        } else {
            None
        }
    }
    pub fn text(&self) -> Option<&str> {
        if let Self::Text(value) = self {
            Some(value)
        } else {
            None
        }
    }
    pub fn number(&self) -> Option<i64> {
        if let Self::Number(value) = self {
            Some(*value)
        } else {
            None
        }
    }
}

pub fn read_file(path: &Path, gzip: bool) -> Option<Value> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader: Box<dyn Read> = if gzip {
        Box::new(flate2::read::GzDecoder::new(file))
    } else {
        Box::new(file)
    };
    let mut data = Vec::new();
    reader
        .by_ref()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut data)
        .ok()?;
    parse(&data)
}

pub fn parse(bytes: &[u8]) -> Option<Value> {
    if bytes.len() > 8 * 1024 * 1024 {
        return None;
    }
    let mut reader = Reader {
        bytes,
        offset: 0,
        budget: 50_000,
    };
    if reader.take(1)?[0] != 10 {
        return None;
    }
    reader.string()?;
    reader.value(10, 0)
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
    budget: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.offset.checked_add(count)?;
        let bytes = self.bytes.get(self.offset..end)?;
        self.offset = end;
        Some(bytes)
    }
    fn length(&mut self) -> Option<usize> {
        usize::try_from(i32::from_be_bytes(self.take(4)?.try_into().ok()?)).ok()
    }
    fn string(&mut self) -> Option<String> {
        let len = u16::from_be_bytes(self.take(2)?.try_into().ok()?) as usize;
        let bytes = self.take(len)?;
        if let Ok(text) = std::str::from_utf8(bytes) {
            return Some(text.to_string());
        }
        let mut units = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            let first = bytes[i];
            if first < 128 {
                units.push(u16::from(first));
                i += 1;
            } else if first & 0xe0 == 0xc0 {
                let next = *bytes.get(i + 1)?;
                if next & 0xc0 != 0x80 {
                    return None;
                }
                units.push((u16::from(first & 31) << 6) | u16::from(next & 63));
                i += 2;
            } else if first & 0xf0 == 0xe0 {
                let b = *bytes.get(i + 1)?;
                let c = *bytes.get(i + 2)?;
                if b & 0xc0 != 0x80 || c & 0xc0 != 0x80 {
                    return None;
                }
                units.push(
                    (u16::from(first & 15) << 12) | (u16::from(b & 63) << 6) | u16::from(c & 63),
                );
                i += 3;
            } else {
                return None;
            }
        }
        Some(String::from_utf16_lossy(&units))
    }
    fn value(&mut self, tag: u8, depth: usize) -> Option<Value> {
        if depth > 32 || self.budget == 0 {
            return None;
        }
        self.budget -= 1;
        Some(match tag {
            1 => Value::Number(i64::from(self.take(1)?[0] as i8)),
            2 => Value::Number(i64::from(i16::from_be_bytes(
                self.take(2)?.try_into().ok()?,
            ))),
            3 => Value::Number(i64::from(i32::from_be_bytes(
                self.take(4)?.try_into().ok()?,
            ))),
            4 => Value::Number(i64::from_be_bytes(self.take(8)?.try_into().ok()?)),
            5 | 6 => {
                self.take(if tag == 5 { 4 } else { 8 })?;
                Value::Skipped
            }
            7 | 11 | 12 => {
                let count = self.length()?;
                self.take(count.checked_mul(match tag {
                    11 => 4,
                    12 => 8,
                    _ => 1,
                })?)?;
                Value::Skipped
            }
            8 => Value::Text(self.string()?),
            9 => {
                let kind = self.take(1)?[0];
                let len = self.length()?;
                if len > self.budget {
                    return None;
                }
                let mut list = Vec::with_capacity(len);
                for _ in 0..len {
                    list.push(self.value(kind, depth + 1)?);
                }
                Value::List(list)
            }
            10 => {
                let mut values = BTreeMap::new();
                loop {
                    let kind = self.take(1)?[0];
                    if kind == 0 {
                        break;
                    }
                    let name = self.string()?;
                    values.insert(name, self.value(kind, depth + 1)?);
                }
                Value::Compound(values)
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_strings_and_rejects_truncated_or_oversized_lists() {
        let world = b"\x0a\x00\x00\x08\x00\x04name\x00\x04Home\x00";
        assert_eq!(
            parse(world).unwrap().get("name").and_then(Value::text),
            Some("Home")
        );
        assert!(parse(&world[..world.len() - 2]).is_none());
        assert!(parse(b"\x0a\x00\x00\x09\x00\x01x\x01\x7f\xff\xff\xff").is_none());
        let emoji = b"\x0a\x00\x00\x08\x00\x01x\x00\x06\xed\xa0\xbd\xed\xb8\x80\x00";
        assert_eq!(
            parse(emoji).unwrap().get("x").and_then(Value::text),
            Some("😀")
        );
    }
}
