use std::collections::BTreeMap;
use std::collections::btree_map;
use std::ops::Deref;
use std::cell::Cell;

#[derive(Debug)]
pub struct PackIndex {
    by_offset: Vec<PackEntry>,
    by_object: BTreeMap<git::ObjectId, usize>
}

#[derive(Debug)]
pub struct PackEntry {
    pub offset: u64,
    pub object: git::ObjectId,
    pub header_len: u8, // length of header
    pub kind: git::ObjectKind,
    pub base_index: Option<usize>, // index into PackIndex::by_offset
    pub stats: PackStats
}

pub struct PackBase<'a> {
    pub base_index: usize,
    pub root_entry: &'a PackEntry
}

#[derive(Debug, Default)]
pub struct PackStats {
    pub referenced: Counter,
    pub referenced_indirect: Counter
}

impl PackIndex {
    pub fn with_capacity(capacity: usize) -> PackIndex {
        PackIndex {
            by_offset: Vec::with_capacity(capacity),
            by_object: BTreeMap::new()
        }
    }
    pub fn push(&mut self, entry: PackEntry) -> &PackEntry {
        let last_offset = self.by_offset.last().map(|e| e.offset);
        assert!(last_offset < Some(entry.offset));

        let idx = self.by_offset.len();
        match self.by_object.entry(entry.object) {
            btree_map::Entry::Vacant(e) => {
                e.insert(self.by_offset.len());
            }
            btree_map::Entry::Occupied(e) => {
                panic!("trying to push {:?}, colliding with existing {:?}", entry, e.get())
            }
        }
        self.by_offset.push(entry);
        &self.by_offset[idx]
    }
    fn find_by_offset(&self, offset: u64) -> Option<usize> {
        self.by_offset.binary_search_by_key(&offset, |e| e.offset).ok()
    }
    fn find_by_object(&self, object: git::ObjectId) -> Option<usize> {
        self.by_object.get(&object).map(|&idx| idx)
    }
    pub fn resolve_base(&self, layer_offsets: &mut Vec<u64>, base: git_pack::DeltaBase) -> Option<PackBase<'_>> {
        let base_index = match base {
            git_pack::DeltaBase::Offset(off)    => self.find_by_offset(off)?,
            git_pack::DeltaBase::Reference(obj) => self.find_by_object(obj)?
        };
        let mut layer = &self.by_offset[base_index];
        increment(&layer.stats.referenced);
        Some(loop {
            match layer.base_index {
                None => break PackBase { base_index, root_entry: layer },
                Some(index) => {
                    layer_offsets.push(layer.offset + layer.header_len as u64);
                    layer = &self.by_offset[index];
                }
            }
            increment(&layer.stats.referenced_indirect);
        })
    }
}

impl Deref for PackIndex {
    type Target = [PackEntry];
    fn deref(&self) -> &[PackEntry] {
        &self.by_offset
    }
}
