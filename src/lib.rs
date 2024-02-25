use std::cell::Cell;
use std::io::{Error, ErrorKind};

pub struct BufferReader<'a> {
    buffer: Cell<&'a [u8]>,
}

impl<'a> BufferReader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        BufferReader {
            buffer: Cell::new(buffer),
        }
    }
}

impl BufferReader<'_> {
    pub fn read_t<T>(&self) -> std::io::Result<&T> {
        let size = std::mem::size_of::<T>();
        self.check_available(size)?;
        let t = unsafe { &*(self.buffer.get().as_ptr() as *const T) };
        unsafe { self.advance(size) };
        Ok(t)
    }
    pub fn read_bytes(&self, len: usize) -> std::io::Result<&[u8]> {
        self.check_available(len)?;
        let bytes = &self.buffer.get()[..len];
        unsafe { self.advance(len) };
        Ok(bytes)
    }
    pub fn get_remaining(&self) -> &[u8] {
        self.buffer.get()
    }
    /// # Safety
    ///
    /// Caller should call `self.check_len(size)` before calling this to check if there is room in the
    /// buffer to advance.
    unsafe fn advance(&self, count: usize) {
        self.buffer.set(&self.buffer.get()[count..]);
    }
    fn check_available(&self, len: usize) -> std::io::Result<()> {
        if len > self.buffer.get().len() {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "BufferReader advance would result in an index that is out of bounds",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read() {
        let hello_world = b"Hello, World!";
        let br = BufferReader::new(hello_world);
        let hello = std::str::from_utf8(br.read_bytes(5).unwrap()).unwrap();
        let world = std::str::from_utf8(br.get_remaining()).unwrap();

        println!("hello: {hello} world: {world}");
    }
}
