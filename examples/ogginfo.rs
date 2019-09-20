//! Prints information about the supplied Ogg Opus files.
#![cfg(feature = "ogg")]

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    use opi::ogg::OggOpusReader;
    use std::{env::args_os, fs::File};

    let args = args_os();
    if args.len() <= 1 {
        eprintln!("Usage: cargo run --example ogginfo <ogg opus file>...");
        return Ok(());
    }

    for filename in args.skip(1) {
        let mut oggopus = OggOpusReader::new(File::open(filename)?)?;
        let mut buf = Vec::<i16>::new();
        println!("{:#?}", oggopus);
        while oggopus.read_samples(&mut buf)? != 0 {}
    }

    Ok(())
}
