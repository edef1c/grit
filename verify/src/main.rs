use std::{mem, io, fs};
use std::io::{Read, BufRead, Seek, SeekFrom};
use flate2::bufread::ZlibDecoder;
use index::{PackIndex, PackEntry};

mod index;

const PACK_PATH: &'static str = "/home/src/android-base/.git/objects/pack/pack-c545e08123f0f1cee2b7e40c4ace577f73213498.pack";

fn main() {
    let mut file = fs::File::open(PACK_PATH).map(io::BufReader::new).unwrap();
    let file_header = gulp::from_reader(&mut file, git_pack::FileHeaderParser::default).unwrap();
    let mut reader = ObjectReader {
        reader: OffsetReader::new(file),
        base: Vec::new(),
        output: Vec::new(),
        layers: Vec::new(),
        index: PackIndex::with_capacity(file_header.count as usize)
    };
    for n in 0..file_header.count {
        print!("\r{} of {} ({}%)", n, file_header.count, n*100/file_header.count);
        let (_entry, _output) = reader.next().unwrap();
    }
}

struct ObjectReader<R: BufRead + Seek> {
    reader: OffsetReader<R>,
    base: Vec<u8>,
    output: Vec<u8>,
    layers: Vec<u64>,
    index: PackIndex
}

impl<R: BufRead + Seek> ObjectReader<R> {
    fn next(&mut self) -> io::Result<(&PackEntry, &[u8])> {
        // read our entry
        let (offset, header, body_offset) = {
            (self.reader.offset()?,
             gulp::from_reader(&mut self.reader, git_pack::EntryHeaderParser::default)?,
             self.reader.offset()?)
        };

        self.layers.clear();
        let (kind, base_index) = match header {
            git_pack::EntryHeader::Object(object_header) => (object_header.kind, None),
            git_pack::EntryHeader::Delta(delta_header) => {
                self.layers.push(body_offset);
                let base = match delta_header.base {
                    git_pack::DeltaBase::Offset(off) => git_pack::DeltaBase::Offset(offset - off),
                    base => base
                };
                let base = match self.index.resolve_base(&mut self.layers, base) {
                    Some(b) => b,
                    None => {
                        println!("known entries: {:?}", &self.index[..]);
                        panic!("can't find base: {:?}", delta_header);
                    }
                };
                let root_offset = base.root_entry.offset + base.root_entry.header_len as u64;
                self.reader.seek(SeekFrom::Start(root_offset))?;
                (base.root_entry.kind, Some(base.base_index))
            }
        };

        self.output.clear();
        ZlibDecoder::new(&mut self.reader).read_to_end(&mut self.output)?;

        for layer_offset in self.layers.drain(..).rev() {
            mem::swap(&mut self.base, &mut self.output);
            let base = io::Cursor::new(&self.base);

            self.reader.seek(SeekFrom::Start(layer_offset))?;
            let delta = io::BufReader::new(ZlibDecoder::new(&mut self.reader));
            self.output.clear();
            git_delta::Reader::new(base, delta)?.read_to_end(&mut self.output)?;
        }

        let object = {
            let size = self.output.len() as u64;
            let mut hasher = git::ObjectHasher::new(git::ObjectHeader { kind, size });
            hasher.update(&self.output);
            hasher.digest()
        };

        let entry = self.index.push(PackEntry {
            offset, object, kind, base_index,
            header_len: (body_offset - offset) as u8,
        });

        Ok((entry, &self.output))
    }
}

struct OffsetReader<R: BufRead + Seek> {
    reader: R,
    offset: Option<u64>
}

impl<R: BufRead + Seek> OffsetReader<R> {
    fn new(reader: R) -> OffsetReader<R> {
        OffsetReader { reader, offset: None }
    }
    fn offset(&mut self) -> io::Result<u64> {
        self.seek(SeekFrom::Current(0))
    }
}

impl<R: BufRead + Seek> Read for OffsetReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let len = self.reader.read(buffer)?;
        self.offset = self.offset.and_then(|off| off.checked_add(len as u64));
        Ok(len)
    }
}

impl<R: BufRead + Seek> BufRead for OffsetReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.reader.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt);
        self.offset = self.offset.and_then(|off| off.checked_add(amt as u64));
    }
}

impl<R: BufRead + Seek> Seek for OffsetReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        Ok(if let (SeekFrom::Current(0), Some(offset)) = (pos, self.offset) {
            offset
        } else {
            let offset = self.reader.seek(pos)?;
            self.offset = Some(offset);
            offset
        })
    }
}
