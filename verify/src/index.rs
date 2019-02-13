#[derive(Debug)]
pub struct PackIndex {
    pub by_offset: Vec<PackEntry>,
    by_object: Vec<usize>
}

#[derive(Debug, Copy, Clone)]
pub struct PackEntry {
    pub offset: u64,
    pub object: git::ObjectId,
    pub header_len: u8, // length of header
    pub kind: git::ObjectKind,
    pub base_index: Option<usize> // index into PackIndex::by_offset
}

pub struct PackBase<'a> {
    pub base_index: usize,
    pub root_entry: &'a PackEntry
}

impl PackIndex {
    pub fn with_capacity(capacity: usize) -> PackIndex {
        PackIndex {
            by_offset: Vec::with_capacity(capacity),
            by_object: Vec::with_capacity(capacity)
        }
    }
    pub fn push(&mut self, entry: PackEntry) -> usize {
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
    pub fn resolve_base(&self, layer_offsets: &mut Vec<u64>, base: git_pack::DeltaBase) -> Option<PackBase<'_>> {
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
