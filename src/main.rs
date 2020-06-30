use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
 
#[derive(Clone)]
struct Value {
    orig: String,
    hex_parsed: Option<Vec<u8>>,
}
 
impl Value {
    fn from_str(s: &str) -> Value {
        let in_hex = hex::decode(s);
        //let hexascii = v.chars().all(|c| c.is_ascii_hexdigit());
 
        let hex_parsed = match in_hex {
            Err(_) => None,
            Ok(bs) => Some(bs),
        };
 
        Value {
            orig: s.to_string(),
            hex_parsed,
        }
    }
}
 
#[derive(Clone)]
struct Content(Vec<(usize, ContentItem)>);
 
#[derive(Clone)]
enum ContentItem {
    Text(String),
    KVS(HashMap<String, Value>),
}
 
#[derive(Clone, Copy, Debug)]
pub enum CodeGen {
    Rust,
    Haskell,
    C,
}
 
enum ParseState {
    Other,
    K(HashMap<String, Value>),
}
 
fn parse_content<P: AsRef<Path>>(file: P) -> std::io::Result<Content> {
    use std::io::Read;
    let contents = {
        let mut file = File::open(file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        contents
    };
 
    let mut parser = ParseState::Other;
    let mut content = Vec::new();
    //let mut h = HashMap::new();
 
    for (line_number, line) in contents.lines().enumerate() {
        let eq_p = line.find('=');
 
        if let Some(eq_pos) = eq_p {
            let k = &line[0..eq_pos].trim();
            let v = &line[eq_pos + 1..].trim();
 
            let ascii = k.len() > 0 && k.chars().all(|c| c.is_ascii_alphanumeric());
            //let even = v.len() % 2 == 0;
            //let hexascii = v.chars().all(|c| c.is_ascii_hexdigit());
 
            //println!(" k=\"{}\", v=\"{}\" ascii={}", k, v, ascii);
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
 
    Ok(Content(content))
}
 
struct Kat<'a> {
    content: &'a Content,
    same_structure: Option<Vec<String>>,
}
 
fn analyze_content(content: &Content) -> Kat {
    let mut its = content.0.iter();
 
    let keys = its.find_map(|(_, ci)| match ci {
        ContentItem::KVS(kvs) => {
            let mut ks = kvs.keys().cloned().collect::<Vec<_>>();
            ks.sort();
            Some(ks)
        }
        _ => None,
    });
 
    match keys {
        None => Kat {
            content,
            same_structure: None,
        },
        Some(expected_keys) => {
            let mut same_structure = true;
            for (line, k) in its {
                match k {
                    ContentItem::Text(_) => {}
                    ContentItem::KVS(kvs) => {
                        let mut keys = kvs.keys().cloned().collect::<Vec<_>>();
                        keys.sort();
                        if keys != expected_keys {
                            println!(
                                "not same structure\n{:?}\n{:?} {}",
                                keys, expected_keys, line
                            );
                            same_structure = false;
                            break;
                        }
                    }
                }
            }
 
            let same_structure = if same_structure {
                Some(expected_keys)
            } else {
                None
            };
 
            Kat {
                content,
                same_structure,
            }
        }
    }
}
 
fn generate_code<'a>(kat: &Kat<'a>, codegen: CodeGen) {
    let with_struct = if let Some(ref expected_keys) = kat.same_structure {
        println!("#[derive(Clone)]");
        println!("struct KV {}", '{');
        for k in expected_keys.iter() {
            println!("    {}: String,", &k);
        }
        println!("{}", '}');
        true
    } else {
        false
    };
 
    let mut kv_number = 0;
    for (_, k) in kat.content.0.iter() {
        match &k {
            ContentItem::Text(t) => {
                println!("// {}", t);
            }
            ContentItem::KVS(kvs) => {
                if with_struct {
                    println!("const kat{}: KV = KV {{", kv_number);
                    for (k, v) in kvs.iter() {
                        let value = match v.hex_parsed {
                            None => format!("\"{}\"", v.orig),
                            Some(ref bs) => {
                                let mut s = String::new();
                                s.push('&');
                                s.push('[');
                                for b in bs.iter() {
                                    s.push_str(&format!("0x{:02x}, ", b))
                                }
                                s.push(']');
                                s
                            }
                        };
                        println!("    {} = {},", k, value);
                    }
                    println!("}};")
                } else {
                    for (k, v) in kvs.iter() {
                        println!("{} = v;", k);
                    }
                }
                kv_number += 1
            }
        }
    }
}
 
fn main() {
    let content = parse_content("kat").expect("content");
    let kat = analyze_content(&content);
    generate_code(&kat, CodeGen::Rust)
}
