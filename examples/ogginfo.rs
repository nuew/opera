//! Prints information about the supplied Ogg Opus files.

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    use opera::ogg::OggOpusReader;
    use std::{env::args_os, fs::File};

    let args = args_os();
    if args.len() <= 1 {
        eprintln!("Usage: cargo run --example ogginfo <ogg opus file>...");
        return Ok(());
    }

    for filename in args.skip(1) {
        println!("{:#?}", OggOpusReader::new(File::open(filename)?)?);
    }

    Ok(())
}
