use failure::Fail;
use std::io;

#[derive(Debug, Fail)]
pub enum IoError<E: Fail> {
    #[fail(display = "parse error: {}", _0)]
    Parse(#[fail(cause)] E),
    #[fail(display = "IO error: {}", _0)]
    Io(#[fail(cause)] io::Error),
    #[fail(display = "parse error: unexpected EOF")]
    UnexpectedEof
}

impl<E: Fail> From<IoError<E>> for io::Error {
    fn from(e: IoError<E>) -> io::Error {
        match e {
            IoError::Parse(e) => io::Error::new(io::ErrorKind::InvalidInput, e.compat()),
            IoError::Io(e) => e,
            IoError::UnexpectedEof => io::Error::new(io::ErrorKind::UnexpectedEof, e.compat())
        }
    }
}

pub type IoResult<T, E> = std::result::Result<T, IoError<E>>;

pub fn from_reader<P: crate::Parse, R: io::BufRead, F: FnOnce() -> P>(mut reader: R, construct: F) -> IoResult<P::Output, P::Err> {
    match next_from_reader(reader, construct) {
        Err(e) => Err(e),
        Ok(None) => Err(IoError::UnexpectedEof::<P::Err>.into()),
        Ok(Some(v)) => Ok(v)
    }
}

pub fn next_from_reader<P: crate::Parse, R: io::BufRead, F: FnOnce() -> P>(mut reader: R, construct: F) -> IoResult<Option<P::Output>, P::Err> {
    let mut parser = Err(construct);
    loop {
        let buf = reader.fill_buf().map_err(IoError::Io)?;
        if buf.len() == 0 {
            return match parser {
                Err(_) => Ok(None),
                Ok(_)  => Err(IoError::UnexpectedEof)
            };
        }
        match parser.unwrap_or_else(|f| f()).parse(buf) {
            crate::Result::Incomplete(p) => {
                parser = Ok(p);
                let len = buf.len();
                reader.consume(len);
            }
            crate::Result::Err(e) => return Err(IoError::Parse(e)),
            crate::Result::Ok(v, tail) => {
                let len = buf.len() - tail.len();
                reader.consume(len);
                return Ok(Some(v));
            }
        }
    }
}
