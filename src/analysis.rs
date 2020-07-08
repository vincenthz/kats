use std::collections::HashMap;

#[derive(Clone, Debug, Copy)]
pub enum T {
    Integer(usize),
    String,
    Bytes(usize),
}

impl T {
    pub fn unify(&self, other: &Self) -> Result<T, String> {
        match (self, other) {
            (T::Integer(b1), T::Integer(b2)) => Ok(T::Integer(std::cmp::max(*b1, *b2))),
            (T::String, T::String) => Ok(T::String),
            (T::Bytes(b1), T::Bytes(b2)) => Ok(T::Bytes(std::cmp::max(*b1, *b2))),
            //(T::Integer(b1), T::Bytes(b2)) => Ok(T::Bytes(std::cmp::max(*b1, *b2))),
            _ => Err("cannot unify".to_string()),
        }
    }
}

#[derive(Clone)]
pub struct Content(pub Vec<(usize, ContentItem)>);

#[derive(Clone)]
pub enum ContentItem {
    Text(String),
    KVS(HashMap<String, Value>),
}

#[derive(Clone)]
pub struct Value {
    pub orig: String,
    pub hex_parsed: Option<Vec<u8>>,
}

impl Value {
    fn from_str(s: &str) -> Value {
        let in_hex = hex::decode(s);

        let hex_parsed = match in_hex {
            Err(_) => None,
            Ok(bs) => Some(bs),
        };

        Value {
            orig: s.to_string(),
            hex_parsed,
        }
    }

    fn is_integral(&self) -> Option<usize> {
        if self.orig.starts_with("0x") {
            if self.orig.chars().skip(2).all(|c| c.is_ascii_hexdigit()) {
                Some((self.orig.len() - 2) * 4)
            } else {
                None
            }
        } else if self.orig.starts_with("0b") {
            if self.orig.chars().skip(2).all(|c| c == '0' || c == '1') {
                Some(self.orig.len())
            } else {
                None
            }
        } else if self.orig.chars().all(|c| c.is_digit(10)) {
            use std::str::FromStr;
            let n = num_bigint::BigUint::from_str(&self.orig).unwrap();
            Some(n.bits() as usize)
        } else {
            None
        }
    }

    fn find_type(&self) -> T {
        match self.is_integral() {
            None => {
                if let Some(hp) = &self.hex_parsed {
                    T::Bytes(hp.len())
                } else {
                    T::String
                }
            }
            Some(bits) => T::Integer(bits),
        }
    }
}

enum ParseState {
    Other,
    K(HashMap<String, Value>),
}

pub fn parse_content(data: &str) -> Content {
    let mut parser = ParseState::Other;
    let mut content = Vec::new();

    for (line_number, line) in data.lines().enumerate() {
        let eq_p = line.find('=');

        if let Some(eq_pos) = eq_p {
            let k = &line[0..eq_pos].trim();
            let v = &line[eq_pos + 1..].trim();

            let ascii = k.len() > 0 && k.chars().all(|c| c.is_ascii_alphanumeric());
            if ascii {
                match parser {
                    ParseState::Other => {
                        let mut h: HashMap<String, Value> = HashMap::new();
                        h.insert(k.to_string(), Value::from_str(v));
                        parser = ParseState::K(h)
                    }
                    ParseState::K(ref mut h) => {
                        h.insert(k.to_string(), Value::from_str(v));
                    }
                }
            } else {
                match parser {
                    ParseState::Other => {
                        content.push((line_number, ContentItem::Text(line.to_string())));
                    }
                    ParseState::K(h) => {
                        content.push((line_number, ContentItem::KVS(h)));
                        parser = ParseState::Other;
                    }
                }
            }
        } else {
            match parser {
                ParseState::K(h) => {
                    content.push((line_number, ContentItem::KVS(h)));
                }
                _ => {}
            };
            if line.trim() != "" {
                content.push((line_number, ContentItem::Text(line.to_string())));
            }
            parser = ParseState::Other
        }
    }

    // flush last one if necessary
    match parser {
        ParseState::K(h) => {
            content.push((0, ContentItem::KVS(h)));
        }
        _ => {}
    };

    Content(content)
}

pub struct Kat<'a> {
    pub content: &'a Content,
    pub same_structure: Option<HashMap<String, T>>,
}

pub fn analyze_content(content: &Content) -> Kat {
    let mut its = content.0.iter();

    let state_keys = its.find_map(|(_, ci)| match ci {
        ContentItem::KVS(kvs) => {
            let mut ks = HashMap::new();
            for (k, v) in kvs.iter() {
                let ty = v.find_type();
                let r = ks.insert(k.clone(), ty);
                if r.is_some() {
                    panic!("duplicated key {}", k)
                }
            }
            Some(ks)
        }
        _ => None,
    });

    let mut state_keys = if let Some(state_keys) = state_keys {
        state_keys
    } else {
        return Kat {
            content,
            same_structure: None,
        };
    };

    for (line, k) in its {
        match k {
            ContentItem::Text(_) => {}
            ContentItem::KVS(kvs) => {
                if state_keys.keys().len() != kvs.keys().len() {
                    println!(
                        "not the same number of keys, expecting {} keys, but {} keys found",
                        state_keys.len(),
                        kvs.len(),
                    );
                }

                for (k, v) in kvs.iter() {
                    match state_keys.get_mut(k) {
                        None => {
                            println!("unexpected key {} line {}", k, line);
                            break;
                        }
                        Some(state_v) => match state_v.unify(&v.find_type()) {
                            Ok(r) => *state_v = r,
                            Err(e) => {
                                println!("key {} has non unifying value {} at line {} (expecting {:?} compatible)", k, v.orig, line, state_v);
                                break;
                            }
                        },
                    }
                }
            }
        }
    }

    Kat {
        content,
        same_structure: Some(state_keys),
    }
}
