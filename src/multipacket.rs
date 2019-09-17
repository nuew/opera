use crate::{
    channel::MappingTable,
    error::Result,
    packet::{Decoder as PktDecoder, Packet},
    sample::{Sample, Samples},
};
use std::vec::IntoIter;

#[derive(Debug, Clone)]
pub struct Multipacket<'a> {
    packets: IntoIter<Packet<'a>>,
}

impl<'a> Multipacket<'a> {
    pub fn new<T>(data: &'a [u8], mapping_table: &T) -> Result<Multipacket<'a>>
    where
        T: ?Sized + MappingTable,
    {
        let streams = mapping_table.streams();
        Ok(Multipacket {
            packets: (0..streams)
                .scan(data, |data, i| {
                    match Packet::new_with_framing(data, i != streams - 1) {
                        Ok((packet, new_data)) => {
                            *data = new_data;
                            Ok(packet)
                        }
                        Err(err) => Err(err),
                    }
                    .into()
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter(),
        })
    }
}

impl<'a> Iterator for Multipacket<'a> {
    type Item = Packet<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.packets.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.packets.size_hint()
    }

    fn count(self) -> usize {
        self.packets.count()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Decoder {
    decoder: PktDecoder,
}

impl Decoder {
    fn new(sample_rate: u32, channels: u8) -> Decoder {
        Decoder {
            decoder: PktDecoder::new(sample_rate, channels),
        }
    }

    pub fn decode<'a, S, T>(
        &mut self,
        multipacket: Option<Multipacket<'a>>,
        buf: &mut S,
    ) -> Result<()>
    where
        S: Samples<T>,
        T: Sample,
    {
        if let Some(multipacket) = multipacket {
            let mut idecs: Vec<Vec<T>> = Vec::with_capacity(multipacket.size_hint().1.unwrap_or(0));
            for packet in multipacket {
                idecs.push(Vec::new());
                self.decoder.decode(Some(packet), idecs.last_mut().unwrap());
            }

            // TODO merge idecs
            unimplemented!()
        } else {
            // TODO packet loss concealment
            unimplemented!()
        }
    }
}
