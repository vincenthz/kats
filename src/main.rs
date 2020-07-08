use std::fs::File;
use std::path::Path;

mod analysis;
mod rust;

use analysis::Kat;

#[derive(Clone, Copy, Debug)]
pub enum CodeGen {
    Rust,
    Haskell,
    C,
}

fn generate_code<'a>(kat: &Kat<'a>, codegen: CodeGen) {
    match codegen {
        CodeGen::Rust => rust::generate(kat),
        _ => panic!("code generator not implemented"),
    }
}

pub fn from_file<P: AsRef<Path>>(file: P) -> std::io::Result<analysis::Content> {
    use std::io::Read;
    let contents = {
        let mut file = File::open(file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        contents
    };
    Ok(analysis::parse_content(&contents))
}

fn main() {
    let content = from_file("kat").expect("content");
    let kat = analysis::analyze_content(&content);
    generate_code(&kat, CodeGen::Rust)
}
