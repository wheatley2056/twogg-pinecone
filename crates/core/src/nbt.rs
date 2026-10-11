//! Minimaler NBT-Leser (Minecrafts Binärformat, Big Endian), z. B. für `servers.dat`.

use std::collections::HashMap;

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<Tag>),
    Compound(HashMap<String, Tag>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Tag {
    pub fn get(&self, key: &str) -> Option<&Tag> {
        match self {
            Tag::Compound(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Tag::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Tag]> {
        match self {
            Tag::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        Some(match self {
            Tag::Byte(v) => *v as i64,
            Tag::Short(v) => *v as i64,
            Tag::Int(v) => *v as i64,
            Tag::Long(v) => *v,
            _ => return None,
        })
    }
}

/// Grenze gegen kaputte oder böswillige Dateien (riesige Längenangaben, tiefe Verschachtelung).
const MAX_DEPTH: usize = 64;
const MAX_LEN: usize = 16 * 1024 * 1024;

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

fn invalid(msg: &str) -> Error {
    Error::Invalid(format!("Ungültige NBT-Daten: {msg}"))
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.data.len())
            .ok_or_else(|| invalid("Datei zu kurz"))?;
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn i16(&mut self) -> Result<i16> {
        Ok(i16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn len(&mut self) -> Result<usize> {
        let n = self.i32()?;
        usize::try_from(n)
            .ok()
            .filter(|&n| n <= MAX_LEN)
            .ok_or_else(|| invalid("unplausible Länge"))
    }

    fn string(&mut self) -> Result<String> {
        let n = self.i16()? as u16 as usize;
        // NBT nutzt „Modified UTF-8“; für Servernamen genügt normales UTF-8 mit Ersatzzeichen.
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }

    fn payload(&mut self, kind: u8, depth: usize) -> Result<Tag> {
        if depth > MAX_DEPTH {
            return Err(invalid("zu tief verschachtelt"));
        }
        Ok(match kind {
            1 => Tag::Byte(self.u8()? as i8),
            2 => Tag::Short(self.i16()?),
            3 => Tag::Int(self.i32()?),
            4 => Tag::Long(self.i64()?),
            5 => Tag::Float(f32::from_bits(self.i32()? as u32)),
            6 => Tag::Double(f64::from_bits(self.i64()? as u64)),
            7 => {
                let n = self.len()?;
                Tag::ByteArray(self.take(n)?.iter().map(|&b| b as i8).collect())
            }
            8 => Tag::String(self.string()?),
            9 => {
                let item = self.u8()?;
                let n = self.len()?;
                let mut items = Vec::with_capacity(n.min(1024));
                for _ in 0..n {
                    items.push(self.payload(item, depth + 1)?);
                }
                Tag::List(items)
            }
            10 => {
                let mut map = HashMap::new();
                loop {
                    let kind = self.u8()?;
                    if kind == 0 {
                        break;
                    }
                    let name = self.string()?;
                    map.insert(name, self.payload(kind, depth + 1)?);
                }
                Tag::Compound(map)
            }
            11 => {
                let n = self.len()?;
                Tag::IntArray((0..n).map(|_| self.i32()).collect::<Result<_>>()?)
            }
            12 => {
                let n = self.len()?;
                Tag::LongArray((0..n).map(|_| self.i64()).collect::<Result<_>>()?)
            }
            other => return Err(invalid(&format!("unbekannter Tag-Typ {other}"))),
        })
    }
}

/// Liest unkomprimiertes NBT mit einem benannten Wurzel-Compound (wie `servers.dat`).
pub fn read(data: &[u8]) -> Result<Tag> {
    let mut r = Reader { data, pos: 0 };
    if r.u8()? != 10 {
        return Err(invalid("Wurzel ist kein Compound"));
    }
    r.string()?;
    r.payload(10, 0)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Baut NBT-Bytes für Tests.
    pub struct Writer(pub Vec<u8>);

    impl Writer {
        pub fn named(&mut self, kind: u8, name: &str) -> &mut Self {
            self.0.push(kind);
            self.str(name)
        }
        pub fn str(&mut self, s: &str) -> &mut Self {
            self.0.extend((s.len() as u16).to_be_bytes());
            self.0.extend(s.as_bytes());
            self
        }
        pub fn byte(&mut self, b: u8) -> &mut Self {
            self.0.push(b);
            self
        }
        pub fn int(&mut self, i: i32) -> &mut Self {
            self.0.extend(i.to_be_bytes());
            self
        }
    }

    /// `servers.dat` wie von Minecraft geschrieben: {servers: [{name, ip, hidden?}, …]}.
    pub fn servers_dat(servers: &[(&str, &str, bool)]) -> Vec<u8> {
        let mut w = Writer(Vec::new());
        w.named(10, "")
            .named(9, "servers")
            .byte(10)
            .int(servers.len() as i32);
        for (name, ip, hidden) in servers {
            w.named(8, "name").str(name).named(8, "ip").str(ip);
            if *hidden {
                w.named(1, "hidden").byte(1);
            }
            w.byte(0);
        }
        w.byte(0);
        w.0
    }

    #[test]
    fn reads_servers_dat() {
        let data = servers_dat(&[
            ("TwoGG", "mc.twogg.de", false),
            ("Lokal", "localhost:25566", true),
        ]);
        let root = read(&data).unwrap();
        let servers = root.get("servers").unwrap().as_list().unwrap();
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].get("name").unwrap().as_str(), Some("TwoGG"));
        assert_eq!(
            servers[1].get("ip").unwrap().as_str(),
            Some("localhost:25566")
        );
        assert_eq!(servers[1].get("hidden").unwrap().as_i64(), Some(1));
    }

    #[test]
    fn rejects_garbage() {
        assert!(read(&[]).is_err());
        assert!(
            read(&[10, 0, 0, 9, 0, 1, b'x', 10, 0x7f, 0xff, 0xff, 0xff]).is_err(),
            "riesige Liste"
        );
        assert!(read(&[8, 0, 0]).is_err(), "Wurzel kein Compound");
        let mut deep = vec![10, 0, 0];
        for _ in 0..100 {
            deep.extend([10, 0, 1, b'a']);
        }
        assert!(read(&deep).is_err(), "zu tief");
    }
}
