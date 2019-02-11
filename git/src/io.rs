use std::io;
use crate::{ObjectHasher, ObjectId};

impl io::Write for ObjectHasher {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.update(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct ObjectWriter<W: io::Write> {
    pub hasher: ObjectHasher,
    pub writer: W
}

impl<W: io::Write> ObjectWriter<W> {
    pub fn digest(self) -> ObjectId {
        self.hasher.digest()
    }
}

impl<W: io::Write> io::Write for ObjectWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
