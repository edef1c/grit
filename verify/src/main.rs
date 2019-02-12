use std::{io, fs};
use std::io::{Read, BufRead, Write, Seek, SeekFrom};
use std::fmt::Write as FmtWrite;

const PACK_PATH: &'static str = "/home/edef/src/github.com/edef1c/libfringe/.git/objects/pack/pack-b452a7d6bcc41ff3e93d12ef285a17c9c04c9804.pack";

fn full_path_for_object_id(object_id: git::ObjectId) -> String {
    format!("/home/edef/src/github.com/edef1c/libfringe-unpacked/.git/objects/{}", path_for_object_id(object_id))
}

fn main() {
    let mut r = fs::File::open(PACK_PATH).map(io::BufReader::new).unwrap();

    let file_header = gulp::from_reader(&mut r, git_pack::FileHeaderParser::default).unwrap();
    writeln!(io::stderr(), "{:?}", file_header).unwrap();
    let mut objects = PackIndex::with_capacity(file_header.count as usize);

    for _ in 0..file_header.count {
        let position = r.seek(SeekFrom::Current(0)).unwrap();
        let entry_header = gulp::from_reader(&mut r, git_pack::EntryHeaderParser::default).unwrap();
        writeln!(io::stderr(), "{:?}", entry_header).unwrap();

        let mut body = flate2::bufread::ZlibDecoder::new(&mut r);
        let mut delta_body;
        let (kind, size, mut body): (git::ObjectKind, u64, &mut Read) = match entry_header {
            git_pack::EntryHeader::Object(object_header) => {
                (object_header.kind, object_header.size, &mut body)
            }
            git_pack::EntryHeader::Delta(delta) => {
                let (base_id, kind) = match delta.base {
                    git_pack::DeltaBase::Reference(base) => {
                        match objects.find_by_id(base) {
                            Some(entry) => (base, entry.kind),
                            None => panic!("couldn't find base object {}", base)
                        }
                    },
                    git_pack::DeltaBase::Offset(base) => {
                        let base_position = position - base;
                        match objects.find_by_offset(base_position) {
                            Some(entry) => (entry.id, entry.kind),
                            None => panic!("couldn't find base object at {}", base_position)
                        }
                    }
                };
                let base_path = full_path_for_object_id(base_id);
                let base = {
                    let mut r = fs::File::open(base_path).map(flate2::read::ZlibDecoder::new).map(io::BufReader::new).unwrap();
                    r.read_until(0, &mut Vec::new()).unwrap();
                    let mut buf = Vec::new();
                    r.read_to_end(&mut buf).unwrap();
                    io::Cursor::new(buf)
                };
                delta_body = git_delta::Reader::new(base, io::BufReader::new(body)).unwrap();
                let size = delta_body.header().result_len;
                (kind, size, &mut delta_body)
            }
        };

        let object_id = {
            let hasher = git::ObjectHasher::new(git::ObjectHeader { kind, size });
            let mut writer = git::ObjectWriter { hasher, writer: io::sink() };
            io::copy(&mut body, &mut writer).unwrap();
            writer.digest()
        };

        objects.push(PackIndexEntry { id: object_id, offset: position, kind });
        let object_path = full_path_for_object_id(object_id);
        fs::File::open(object_path).unwrap();
    }
}

struct PackIndex {
    by_offset: Vec<PackIndexEntry>,
    by_id: Vec<usize>
}

#[derive(Copy, Clone, Debug)]
struct PackIndexEntry {
    offset: u64,
    id: git::ObjectId,
    kind: git::ObjectKind
}

impl PackIndex {
    fn with_capacity(capacity: usize) -> PackIndex {
        PackIndex {
            by_offset: Vec::with_capacity(capacity),
            by_id: Vec::with_capacity(capacity)
        }
    }
    fn push(&mut self, entry: PackIndexEntry) {
        let last_offset = self.by_offset.last().map(|e| e.offset);
        assert!(last_offset < Some(entry.offset));

        let id_idx = self.by_id.binary_search_by_key(&entry.id, |&idx| self.by_offset[idx].id).err().unwrap();
        self.by_id.insert(id_idx, self.by_offset.len());
        self.by_offset.push(entry);
    }
    fn find_by_offset(&self, offset: u64) -> Option<&PackIndexEntry> {
        self.by_offset.binary_search_by_key(&offset, |e| e.offset).ok().map(|idx| &self.by_offset[idx])
    }
    fn find_by_id(&self, id: git::ObjectId) -> Option<&PackIndexEntry> {
        self.by_id.binary_search_by_key(&id, |&idx| self.by_offset[idx].id).ok().map(|idx| &self.by_offset[idx])
    }
}

fn path_for_object_id(git::ObjectId(bytes): git::ObjectId) -> String {
    let mut result = String::new();
    write!(result, "{:02x}/", bytes[0]).unwrap();
    for &b in &bytes[1..] {
        write!(result, "{:02x}", b).unwrap();
    }
    result
}
