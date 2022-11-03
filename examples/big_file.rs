use dwparser::DWSyntax;
use std::{
    fs::File, io::{self, BufReader, Read}, time::Instant
};

fn from_file(file_path: &str) -> io::Result<String> {
    let file = File::open(file_path)?;

    let mut rd = BufReader::new(file);

    //检查UTF-8 BOM
    let mut bom = [0; 3];
    rd.read_exact(&mut bom)?;
    if &bom != &[0xEF, 0xBB, 0xBF] {
        rd.seek_relative(-3)?;
    }

    let mut syn = String::new();
    rd.read_to_string(&mut syn)?;

    Ok(syn)
}

fn main() {
    let dwsyn = from_file("assets/big_file.srd").unwrap();
    let now = Instant::now();
    DWSyntax::parse(&dwsyn).unwrap();
    println!("elapsed: {}ms", now.elapsed().as_millis());
}
