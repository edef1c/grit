use std::{mem, io, fs};
use std::io::{Read, BufRead, Seek, SeekFrom};
use flate2::bufread::ZlibDecoder;


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
                        println!("known entries: {:?}", self.index.by_offset);
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

        let idx = self.index.push(PackEntry {
            offset, object, kind, base_index,
            header_len: (body_offset - offset) as u8,
        });

        Ok((&self.index.by_offset[idx], &self.output))
    }
}

#[derive(Debug)]
struct PackIndex {
    by_offset: Vec<PackEntry>,
    by_object: Vec<usize>
}

#[derive(Debug, Copy, Clone)]
struct PackEntry {
    offset: u64,
    object: git::ObjectId,
    header_len: u8, // length of header
    kind: git::ObjectKind,
    base_index: Option<usize> // index into PackIndex::by_offset
}

struct PackBase<'a> {
    base_index: usize,
    root_entry: &'a PackEntry
}

impl PackIndex {
    fn with_capacity(capacity: usize) -> PackIndex {
        PackIndex {
            by_offset: Vec::with_capacity(capacity),
            by_object: Vec::with_capacity(capacity)
        }
    }
    fn push(&mut self, entry: PackEntry) -> usize {
        let last_offset = self.by_offset.last().map(|e| e.offset);
        assert!(last_offset < Some(entry.offset));

        let idx = self.by_offset.len();
        match self.by_object.binary_search_by_key(&entry.object, |&idx| self.by_offset[idx].object) {
            Err(obj_idx) => self.by_object.insert(obj_idx, self.by_offset.len()),
            Ok(obj_idx) => panic!("trying to push {:?}, colliding with existing {:?}", entry, self.by_offset[self.by_object[obj_idx]])
        }
        self.by_offset.push(entry);
        idx
    }
    fn find_by_offset(&self, offset: u64) -> Option<usize> {
        self.by_offset.binary_search_by_key(&offset, |e| e.offset).ok()
    }
    fn find_by_object(&self, object: git::ObjectId) -> Option<usize> {
        self.by_object.binary_search_by_key(&object, |&idx| self.by_offset[idx].object).ok()
    }
    fn resolve_base(&self, layer_offsets: &mut Vec<u64>, base: git_pack::DeltaBase) -> Option<PackBase<'_>> {
        let base_index = match base {
            git_pack::DeltaBase::Offset(off)    => self.find_by_offset(off)?,
            git_pack::DeltaBase::Reference(obj) => self.find_by_object(obj)?
        };
        let mut entry_index = base_index;
        Some(loop {
            let layer = &self.by_offset[entry_index];
            entry_index = match layer.base_index {
                None => break PackBase { base_index, root_entry: layer },
                Some(index) => index
            };
            layer_offsets.push(layer.offset + layer.header_len as u64);
        })
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
