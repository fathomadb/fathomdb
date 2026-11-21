use serde_json::{json, Value};
use std::time::{Duration, Instant};

struct Scan<F> {
    read: F,
    total: usize,
    base: usize,
    cache: Vec<u8>,
    fetched: usize,
    deadline: Instant,
    byte_budget: usize,
}
impl<F: FnMut(usize, usize) -> Result<Vec<u8>, &'static str>> Scan<F> {
    fn byte(&mut self, pos: usize) -> Result<Option<u8>, &'static str> {
        if pos >= self.total {
            return Ok(None);
        }
        if pos < self.base || pos >= self.base + self.cache.len() {
            if self.fetched >= self.byte_budget || Instant::now() >= self.deadline {
                return Err("budget_exhausted");
            }
            self.base = (pos / 65536) * 65536;
            self.cache = (self.read)(self.base, 65536.min(self.byte_budget - self.fetched))?;
            self.fetched += self.cache.len();
        }
        self.cache.get(pos - self.base).copied().map(Some).ok_or("unavailable")
    }
    fn ws(&mut self, mut p: usize) -> Result<usize, &'static str> {
        while self.byte(p)?.is_some_and(|b| b" \t\r\n".contains(&b)) {
            p += 1;
        }
        Ok(p)
    }
    fn string(&mut self, mut p: usize) -> Result<usize, &'static str> {
        if self.byte(p)? != Some(b'"') {
            return Err("not_json");
        }
        p += 1;
        loop {
            match self.byte(p)?.ok_or("not_json")? {
                b'"' => return Ok(p + 1),
                b'\\' => {
                    p += 1;
                    match self.byte(p)?.ok_or("not_json")? {
                        b'u' => {
                            for _ in 0..4 {
                                p += 1;
                                if !self.byte(p)?.is_some_and(|b| b.is_ascii_hexdigit()) {
                                    return Err("not_json");
                                }
                            }
                        }
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                        _ => return Err("not_json"),
                    }
                }
                0..=31 => return Err("not_json"),
                _ => {}
            }
            p += 1;
        }
    }
    fn value(&mut self, p: usize, depth: usize) -> Result<(&'static str, usize), &'static str> {
        if depth > 128 {
            return Err("budget_exhausted");
        }
        let p = self.ws(p)?;
        match self.byte(p)?.ok_or("not_json")? {
            b'"' => Ok(("string", self.string(p)?)),
            b'{' | b'[' => {
                let object = self.byte(p)? == Some(b'{');
                let end = if object { b'}' } else { b']' };
                let mut q = self.ws(p + 1)?;
                if self.byte(q)? == Some(end) {
                    return Ok((if object { "object" } else { "array" }, q + 1));
                }
                loop {
                    if object {
                        q = self.string(q)?;
                        q = self.ws(q)?;
                        if self.byte(q)? != Some(b':') {
                            return Err("not_json");
                        }
                        q += 1;
                    }
                    q = self.value(q, depth + 1)?.1;
                    q = self.ws(q)?;
                    match self.byte(q)? {
                        Some(b',') => {
                            q = self.ws(q + 1)?;
                        }
                        Some(b) if b == end => {
                            return Ok((if object { "object" } else { "array" }, q + 1))
                        }
                        _ => return Err("not_json"),
                    }
                }
            }
            b't' | b'f' | b'n' => {
                let token = match self.byte(p)? {
                    Some(b't') => "true",
                    Some(b'f') => "false",
                    _ => "null",
                };
                for (i, b) in token.bytes().enumerate() {
                    if self.byte(p + i)? != Some(b) {
                        return Err("not_json");
                    }
                }
                Ok((if token == "null" { "null" } else { "boolean" }, p + token.len()))
            }
            b'-' | b'0'..=b'9' => {
                let mut q = p;
                if self.byte(q)? == Some(b'-') {
                    q += 1;
                }
                if self.byte(q)? == Some(b'0') {
                    q += 1;
                } else {
                    if !self.byte(q)?.is_some_and(|b| b.is_ascii_digit() && b != b'0') {
                        return Err("not_json");
                    }
                    while self.byte(q)?.is_some_and(|b| b.is_ascii_digit()) {
                        q += 1;
                    }
                }
                if self.byte(q)? == Some(b'.') {
                    q += 1;
                    let first = q;
                    while self.byte(q)?.is_some_and(|b| b.is_ascii_digit()) {
                        q += 1;
                    }
                    if q == first {
                        return Err("not_json");
                    }
                }
                if matches!(self.byte(q)?, Some(b'e' | b'E')) {
                    q += 1;
                    if matches!(self.byte(q)?, Some(b'+' | b'-')) {
                        q += 1;
                    }
                    let first = q;
                    while self.byte(q)?.is_some_and(|b| b.is_ascii_digit()) {
                        q += 1;
                    }
                    if q == first {
                        return Err("not_json");
                    }
                }
                Ok(("number", q))
            }
            _ => Err("not_json"),
        }
    }
    fn text(&mut self, start: usize, end: usize) -> Result<String, &'static str> {
        let mut bytes = Vec::new();
        for p in start..end.min(start + 4096) {
            bytes.push(self.byte(p)?.ok_or("unavailable")?);
        }
        while !bytes.is_empty() && std::str::from_utf8(&bytes).is_err() {
            bytes.pop();
        }
        String::from_utf8(bytes).map_err(|_| "not_json")
    }
}
pub(crate) fn children<F: FnMut(usize, usize) -> Result<Vec<u8>, &'static str>>(
    read: F,
    total: usize,
    start: usize,
    after: Option<usize>,
) -> Result<Value, &'static str> {
    children_until(
        read,
        total,
        start,
        after,
        Instant::now() + Duration::from_secs(30),
        64 * 1024 * 1024,
    )
}
pub(crate) fn children_until<F: FnMut(usize, usize) -> Result<Vec<u8>, &'static str>>(
    read: F,
    total: usize,
    start: usize,
    after: Option<usize>,
    deadline: Instant,
    byte_budget: usize,
) -> Result<Value, &'static str> {
    let mut s =
        Scan { read, total, base: usize::MAX, cache: vec![], fetched: 0, deadline, byte_budget };
    let root = start == 0;
    let start = s.ws(start)?;
    let (kind, end) = s.value(start, 0)?;
    if root && s.ws(end)? != total {
        return Err("not_json");
    }
    if !matches!(kind, "object" | "array") {
        return Ok(
            json!({"status":"complete","kind":kind,"start":start.to_string(),"end":end.to_string(),"lexeme":s.text(start,end)?,"items":[]}),
        );
    }
    let mut p = s.ws(start + 1)?;
    let mut index = 0;
    let mut items = vec![];
    let mut item_bytes = 0;
    let mut continuation = None;
    let mut found = after.is_none();
    while p < end - 1 {
        if index >= 100000 {
            return Err("budget_exhausted");
        }
        let mut key = None;
        let mut key_range = None;
        if kind == "object" {
            let key_start = p;
            let key_end = s.string(p)?;
            let raw = s.text(key_start, key_end)?;
            if key_end - key_start <= 4096 {
                key = Some(serde_json::from_str::<String>(&raw).map_err(|_| "not_json")?);
            } else {
                key = Some("oversize key (copy exact range)".into());
            }
            key_range = Some(json!({"start":key_start.to_string(),"end":key_end.to_string()}));
            p = s.ws(key_end)? + 1;
            p = s.ws(p)?;
        }
        let (child_kind, child_end) = s.value(p, 1)?;
        if after.is_some_and(|v| v == p) {
            found = true;
        }
        if found {
            if items.len() == 200 {
                continuation = Some(p.to_string());
                break;
            }
            let lexeme = if matches!(child_kind, "object" | "array") {
                None
            } else {
                Some(s.text(p, child_end)?)
            };
            let item = json!({"key":key,"key_range":key_range,"index":index,"kind":child_kind,"start":p.to_string(),"end":child_end.to_string(),"lexeme":lexeme,"preview_truncated":child_end-p>4096,"expandable":matches!(child_kind,"object"|"array")});
            let bytes = serde_json::to_vec(&item).map_err(|_| "storage")?.len();
            if item_bytes + bytes > crate::storage::RESPONSE - 16384 {
                continuation = Some(p.to_string());
                break;
            }
            item_bytes += bytes;
            items.push(item);
        }
        p = s.ws(child_end)?;
        if s.byte(p)? == Some(b',') {
            p = s.ws(p + 1)?;
        }
        index += 1;
    }
    if !found {
        return Err("invalid_range");
    }
    Ok(
        json!({"status":if continuation.is_some(){"partial"}else{"complete"},"kind":kind,"start":start.to_string(),"end":end.to_string(),"items":items,"continuation":continuation,"examined_bytes":s.fetched,"returned":items.len(),"item_bytes":item_bytes,"byte_limit":crate::storage::RESPONSE,"reason":if continuation.is_some(){Some("item_or_serialized_byte_cap")}else{None},"limit":200}),
    )
}
