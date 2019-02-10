use std::{io, fs};
use std::io::{Read, BufRead, Write, Seek, SeekFrom};
use std::fmt::Write as FmtWrite;

const PACK_PATH: &'static str = "/home/edef/src/github.com/edef1c/libfringe/.git/objects/pack/pack-b452a7d6bcc41ff3e93d12ef285a17c9c04c9804.pack";

fn full_path_for_object_id(object_id: git::ObjectId) -> String {
  format!("/home/edef/src/github.com/edef1c/libfringe-unpacked/.git/objects/{}", path_for_object_id(object_id))
}

fn main() {
  let mut r = fs::File::open(PACK_PATH).map(io::BufReader::new).unwrap();
  let mut file_header = gulp::from_reader(&mut r, git_packfile::FileHeaderParser::default).unwrap().unwrap();
  writeln!(io::stderr(), "{:?}", file_header).unwrap();
  let mut objects = PackfileIndex::new();
  while let (position, Some(entry_header)) = (r.seek(SeekFrom::Current(0)).unwrap(), gulp::from_reader(&mut r, git_packfile::EntryHeaderParser::default).unwrap()) {
    writeln!(io::stderr(), "{:?}", entry_header).unwrap();
    let mut body = flate2::bufread::ZlibDecoder::new(&mut r);
    let mut hasher = Sha1Writer(sha1::Sha1::new(), io::sink());
    let mut delta_body;
    let (kind, size, mut body): (git::ObjectKind, u64, &mut Read) = match entry_header {
      git_packfile::EntryHeader::Object(object_header) => {
        (object_header.kind, object_header.size, &mut body)
      }
      git_packfile::EntryHeader::Delta(delta) => {
        let (base_id, kind) = match delta.base {
          git_packfile::DeltaBase::Reference(base) => {
            match objects.find_by_id(base) {
              Some(entry) => (base, entry.kind),
              None => panic!("couldn't find base object {}", base)
            }
          },
          git_packfile::DeltaBase::Offset(base) => {
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
        delta_body = git_delta::DeltaReader::new(base, io::BufReader::new(body)).unwrap().unwrap();
        let size = delta_body.header().result_len;
        (kind, size, &mut delta_body)
      }
    };
    write!(hasher, "{} {}\u{0}", kind.name(), size).unwrap();
    io::copy(&mut body, &mut hasher).unwrap();
    let Sha1Writer(hasher, _) = hasher;
    let object_id = git::ObjectId(hasher.digest().bytes());
    objects.add(PackfileIndexEntry { id: object_id, offset: position, kind });
    let object_path = full_path_for_object_id(object_id);
    fs::File::open(object_path).unwrap();

    file_header.count -= 1;
    if file_header.count == 0 {
      break;
    }
  }
}

struct PackfileIndex {
  objects: Vec<PackfileIndexEntry>,
  by_id: Vec<usize>,
  by_offset: Vec<usize>
}

#[derive(Copy, Clone, Debug)]
struct PackfileIndexEntry {
  id: git::ObjectId,
  offset: u64,
  kind: git::ObjectKind
}

impl PackfileIndex {
  fn new() -> PackfileIndex {
    PackfileIndex {
      objects: Vec::new(),
      by_id: Vec::new(),
      by_offset: Vec::new()
    }
  }
  fn add(&mut self, entry: PackfileIndexEntry) {
    let id_idx = self.by_id.binary_search_by_key(&entry.id, |&idx| self.objects[idx].id).err().unwrap();
    let offset_idx = self.by_offset.binary_search_by_key(&entry.offset, |&idx| self.objects[idx].offset).err().unwrap();
    let idx = self.objects.len();
    self.by_id.insert(id_idx, idx);
    self.by_offset.insert(offset_idx, idx);
    self.objects.push(entry);
  }
  fn find_by_id(&self, id: git::ObjectId) -> Option<&PackfileIndexEntry> {
    self.by_id.binary_search_by_key(&id, |&idx| self.objects[idx].id).ok().map(|idx| &self.objects[idx])
  }
  fn find_by_offset(&self, offset: u64) -> Option<&PackfileIndexEntry> {
    self.by_offset.binary_search_by_key(&offset, |&idx| self.objects[idx].offset).ok().map(|idx| &self.objects[idx])
  }
}

struct Sha1Writer<W: io::Write>(sha1::Sha1, W);

impl<W: io::Write> io::Write for Sha1Writer<W> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    match self.1.write(buf) {
      Ok(n) => {
        self.0.update(&buf[..n]);
        Ok(n)
      }
      Err(e) => Err(e)
    }
  }
  fn flush(&mut self) -> io::Result<()> {
    self.1.flush()
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
