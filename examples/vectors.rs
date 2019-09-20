//! Dumps an Opus test-vector bitstream

use opi::Error;
use std::{error, io::prelude::*};

fn report_error(err: Error, buffer: &[u8]) -> Box<dyn error::Error> {
    eprintln!("PARSE ERROR: {:?} [", err);
    for byte in buffer.into_iter() {
        eprintln!("\t{:3.} ({:#010b}, {:#04x})),", byte, byte, byte);
    }
    eprintln!("]");

    err.into()
}

fn dump<R: Read>(mut reader: R) -> Result<(), Box<dyn error::Error>> {
    use opi::packet::{Decoder, Packet};
    use std::{convert::TryInto, io::ErrorKind};

    const MAX_PACKET: usize = 1500;
    const CHANNELS: u8 = 2;
    const MAX_FRAME_SIZE: usize = 960 * 6;

    let mut frame = [0; 8];
    let mut packet_buf = [0; MAX_PACKET];
    let mut out_buf = [0i16; MAX_FRAME_SIZE * CHANNELS as usize];
    let mut decoder = Decoder::new(48000, CHANNELS);

    loop {
        // get packet framing
        match reader.read_exact(&mut frame) {
            Ok(()) => {}
            Err(ref err) if err.kind() == ErrorKind::UnexpectedEof => break Ok(()),
            err => err?,
        };
        let len = i32::from_be_bytes(frame[0..4].try_into().unwrap()) as usize;
        let enc_final_range = i32::from_be_bytes(frame[4..8].try_into().unwrap());

        println!("== PACKET ==");
        println!("len = {}; enc_final_range = {}", len, enc_final_range);

        // get packet
        let packet = if len != 0 {
            reader.read_exact(&mut packet_buf[0..len])?;

            let packet = Packet::new(&packet_buf[0..len])?;
            println!("{:?}", packet);

            Some(packet)
        } else {
            None
        };

        let samples_len = decoder.decode(packet, &mut &mut out_buf[..]);
        println!();
    }
}

fn main() -> Result<(), Box<dyn error::Error>> {
    use std::{env::args, fs::File, io::BufReader};

    let filename = match args().nth(1) {
        Some(filename) => filename,
        None => {
            eprintln!("Usage: cargo run --example dump <opus self-framed packet bitstream>");
            return Ok(());
        }
    };

    dump(BufReader::new(File::open(filename)?)).into()
}
