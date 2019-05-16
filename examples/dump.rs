//! Dumps a self-framed opus bitstream; for example, the test vectors.

use opera::packet::MalformedPacketError;
use std::{error::Error, io::prelude::*};

fn report_error(err: MalformedPacketError, buffer: &[u8]) -> Box<dyn Error> {
    eprintln!("PARSE ERROR: {:?} [", err);
    for byte in buffer.into_iter() {
        eprintln!("\t{:3.} ({:#010b}, {:#04x})),", byte, byte, byte);
    }
    eprintln!("]");

    err.into()
}

fn dump<R: Read>(mut reader: R) -> Result<(), Box<dyn Error>> {
    use opera::packet::Packet;
    use std::ptr::copy;

    let mut buffer = [0; u16::max_value() as usize];
    let mut read_len = reader.read(&mut buffer)?;
    loop {
        match Packet::new_with_framing(&buffer[..read_len], true) {
            Ok((packet, next)) => {
                println!("{:#?}", packet);

                read_len = next.len();
                unsafe { copy(next.as_ptr(), buffer.as_ptr() as *mut _, next.len()) }
            }
            Err(MalformedPacketError::UnexpectedEof) => {
                let read = reader.read(&mut buffer[read_len..])?;

                if read == 0 {
                    break if read_len == 0 {
                        Ok(())
                    } else {
                        Err(report_error(
                            MalformedPacketError::UnexpectedEof,
                            &buffer[..read_len],
                        ))
                    };
                } else {
                    read_len += read;
                }
            }
            Err(err) => break Err(report_error(err, &buffer[..read_len])),
        };
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    use std::{env::args, fs::File};

    let filename = match args().nth(1) {
        Some(filename) => filename,
        None => {
            eprintln!("Usage: cargo run --example dump <opus self-framed packet bitstream>");
            return Ok(());
        }
    };

    dump(File::open(filename)?).into()
}
