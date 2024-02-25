use std::cell::Cell;
use std::io::{Error, ErrorKind};

pub struct BufferReader<'a> {
    buffer: Cell<&'a [u8]>,
}

impl<'a> BufferReader<'a> {
    /// Returns a new `BufferReader<'a>` for the provided slice.
    pub fn new(slice: &'a [u8]) -> Self {
        BufferReader {
            buffer: Cell::new(slice),
        }
    }
    /// Returns a reference to the next `n` bytes in the slice as a reference to `T`. and then
    /// advances the stream by the size of `T`. Function will fail if the length of the underlying
    /// slice is less than the size of `T`.
    pub fn read_t<T>(&self) -> std::io::Result<&'a T> {
        let size = std::mem::size_of::<T>();
        let slice = self.check_and_advance(size)?;
        Ok(unsafe { &*(slice.as_ptr() as *const T) })
    }
    /// Returns a reference to the next `n` bytes specified by the size parameter. Function will fail
    /// if the length of the underlying slice is less than the size provided.
    pub fn read_bytes(&self, size: usize) -> std::io::Result<&'a [u8]> {
        self.check_and_advance(size)
    }
    /// Returns a reference to the remaining bytes in the slice.
    pub fn get_remaining(self) -> &'a [u8] {
        self.buffer.get()
    }
    /// Checks that there are enough bytes left in the slice to advance the start of the slice position,
    /// and returns a slice from the previous start of the buffer to the new start of the buffer.
    fn check_and_advance(&self, size: usize) -> std::io::Result<&'a [u8]> {
        self.check_available(size)?;
        let slice = self.buffer.get();
        unsafe { self.advance(size) }
        Ok(&slice[..size])
    }
    /// # Safety
    ///
    /// Caller should call `self.check_len(size)` before calling this to check if there is room in the
    /// buffer to advance.
    #[inline(always)]
    unsafe fn advance(&self, size: usize) {
        self.buffer.set(&self.buffer.get()[size..]);
    }
    fn check_available(&self, size: usize) -> std::io::Result<()> {
        if size > self.buffer.get().len() {
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

        assert_eq!(hello, "Hello");
        assert_eq!(world, ", World!");
    }
}
