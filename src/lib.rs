use std::cell::Cell;
use std::io::{Error, ErrorKind};

/// A structure used for getting references to C structures in a contiguous buffer of memory.
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
    /// advances the slice by the size of `T` in bytes. Function will fail if the length of the underlying
    /// slice is less than the size of `T`.
    pub fn read_t<T>(&self) -> std::io::Result<&'a T> {
        let size = std::mem::size_of::<T>();
        let slice = self.check_and_advance(size)?;
        // SAFETY: We know that the buffer passed back from `self.check_and_advance(size)?` is the size
        // of T, so we will assume that it's a valid T. I might make this function unsafe, because the
        // caller should do additional verification that the reference to T that is passed back is valid.
        Ok(unsafe { &*(slice.as_ptr() as *const T) })
    }
    // @TODO: Make this suck less
    /// Returns the value next byte. Function will fail if the length of the underlying slice is less
    /// than 1.
    pub fn read_byte(&self) -> std::io::Result<u8> {
        Ok(self.check_and_advance(1)?[0])
    }
    /// Returns a reference to the next `n` bytes specified by the `len` parameter. Function will fail
    /// if the length of the underlying slice is less than the size provided.
    pub fn read_bytes(&self, len: usize) -> std::io::Result<&'a [u8]> {
        self.check_and_advance(len)
    }
    /// Returns the length of the remaining buffer.
    pub fn len(&self) -> usize {
        self.buffer.get().len()
    }
    /// Returns a reference to the remaining bytes in the slice.
    pub fn get_remaining(self) -> &'a [u8] {
        self.buffer.get()
    }
    /// Checks that there are enough bytes left in the slice to advance the start of the slice position,
    /// and returns a slice from the previous start of the buffer to the new start of the buffer.
    fn check_and_advance(&self, len: usize) -> std::io::Result<&'a [u8]> {
        self.check_available(len)?;
        Ok(self.advance(len))
    }
    /// Advance the start of the buffer by the number of bytes provided by `len`. Returns a slice from
    /// the previous start of the buffer up until the new start of the buffer.
    ///
    /// # Safety
    ///
    /// Caller should call `self.check_available(size)` before calling this to check if there is room in the
    /// buffer to advance.
    #[inline(always)]
    fn advance(&self, len: usize) -> &'a [u8] {
        let buffer = self.buffer.get();
        self.buffer.set(&buffer[len..]);
        &buffer[..len]
    }
    /// Checks if there are enough bytes left in the buffer.
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

        assert_eq!(hello, "Hello");
        assert_eq!(world, ", World!");
    }
}
