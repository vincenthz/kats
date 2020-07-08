use super::analysis::{self, ContentItem, Kat, T};
use num_bigint::BigUint;

#[derive(Clone, Debug, Copy)]
enum RustTy {
    U8,
    U16,
    U32,
    U64,
    U128,
    Bytes,
    String,
}

impl RustTy {
    pub fn as_string(&self) -> String {
        match self {
            RustTy::U8 => "u8".to_string(),
            RustTy::U16 => "u16".to_string(),
            RustTy::U32 => "u32".to_string(),
            RustTy::U64 => "u64".to_string(),
            RustTy::U128 => "u128".to_string(),
            RustTy::String => "String".to_string(),
            RustTy::Bytes => "&'static [u8]".to_string(),
        }
    }
}

impl From<analysis::T> for RustTy {
    fn from(t: analysis::T) -> Self {
        match t {
            T::Integer(b) => {
                if b <= 8 {
                    RustTy::U8
                } else if b <= 16 {
                    RustTy::U16
                } else if b <= 32 {
                    RustTy::U32
                } else if b <= 64 {
                    RustTy::U64
                } else if b <= 128 {
                    RustTy::U128
                } else {
                    RustTy::Bytes
                }
            }
            T::String => RustTy::String,
            T::Bytes(_) => RustTy::Bytes,
        }
    }
}

fn bytes_to_array(bs: &Vec<u8>) -> String {
    let mut s = String::new();
    s.push('&');
    s.push('[');
    for (i, b) in bs.iter().enumerate() {
        if i != 0 {
            s.push_str(", ");
        }
        s.push_str(&format!("0x{:02x}", b))
    }
    s.push(']');
    s
}

pub fn generate<'a>(kat: &Kat<'a>) {
    let mut scope = codegen::Scope::new();
    let with_struct = if let Some(ref expected_keys) = kat.same_structure {
        let struc = scope.new_struct("KV");
        struc.vis("pub");
        struc.derive("Debug");
        struc.derive("Clone");
        for (k, kty) in expected_keys.iter() {
            let field = struc.field(k, RustTy::from(*kty).as_string());
            field.vis("pub");
        }
        Some(expected_keys)
    } else {
        None
    };

    println!("{}", scope.to_string());

    let mut kv_number = 0;
    let len = kat
        .content
        .0
        .iter()
        .filter(|x| {
            if let ContentItem::KVS(_) = x.1 {
                true
            } else {
                false
            }
        })
        .collect::<Vec<_>>()
        .len();
    println!("pub const KATS : [KV; {}] = [", len);
    for (_, k) in kat.content.0.iter() {
        match &k {
            ContentItem::Text(t) => {
                println!("// {}", t);
            }
            ContentItem::KVS(kvs) => {
                if let Some(expected_keys) = with_struct {
                    println!("// KAT {}", kv_number);
                    println!("KV {{");
                    for (k, v) in kvs.iter() {
                        match expected_keys.get(k) {
                            None => unreachable!(),
                            Some(kty) => match kty {
                                T::Integer(sz) => {
                                    use std::str::FromStr;
                                    let n = BigUint::from_str(&v.orig).unwrap();

                                    let mut bytes = n.to_bytes_be();
                                    let output = (sz + 7) / 8;
                                    let pre0 = output - bytes.len();
                                    for _ in 0..pre0 {
                                        bytes.insert(0, 0);
                                    }
                                    let value = bytes_to_array(&bytes);
                                    println!("    {}: {},", k, value);
                                }
                                _ => {
                                    let value = match v.hex_parsed {
                                        None => format!("\"{}\"", v.orig),
                                        Some(ref bs) => bytes_to_array(bs),
                                    };
                                    println!("    {}: {},", k, value);
                                }
                            },
                        }
                    }
                    println!("}},")
                } else {
                    for (k, v) in kvs.iter() {
                        println!("{} = v;", k);
                    }
                }
                kv_number += 1
            }
        }
    }
    println!("];")
}
