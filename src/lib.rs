use std::cell::Cell;
use std::io::{Error, ErrorKind, Read};

/// A structure used for getting references to C structures in a contiguous buffer of memory.
pub struct BufferReader<'a> {
    buffer: Cell<&'a [u8]>,
}

impl Read for BufferReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.check_available(buf.len()) {
            Ok(_) => {
                buf.copy_from_slice(self.advance(buf.len()));
                Ok(buf.len())
            }
            Err(_) => {
                let buffer = self.buffer.get();
                buf.copy_from_slice(buffer);
                Ok(buffer.len())
            }
        }
    }
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
        self.check_available(size)?;
        let slice = self.advance(size);
        // SAFETY: We know that the buffer passed back from `self.check_and_advance(size)?` is the size
        // of T, so we will assume that it's a valid T. I might make this function unsafe, because the
        // caller should do additional verification that the reference to T that is passed back is valid.
        Ok(unsafe { &*(slice.as_ptr() as *const T) })
    }
    /// Returns a reference to the next `n` bytes in the slice as a reference to `T`, Where n is the
    /// size of `T`. Function will fail if there are not enough bytes left in the buffer.
    pub fn peek_t<T>(&self, start: usize) -> std::io::Result<&'a T> {
        let len = std::mem::size_of::<T>();
        let end = start + len;
        self.check_available(end)?;
        let slice = &self.peek_remaining()[start..end];
        // SAFETY: We know that the buffer passed back from `self.check_and_advance(size)?` is the size
        // of T, so we will assume that it's a valid T. I might make this function unsafe, because the
        // caller should do additional verification that the reference to T that is passed back is valid.
        Ok(unsafe { &*(slice.as_ptr() as *const T) })
    }
    /// Returns the value next byte and advances the slice by one. Function will fail if the length
    /// of the underlying slice is less than 1.
    pub fn read_byte(&self) -> std::io::Result<u8> {
        self.check_available(std::mem::size_of::<u8>())?;
        // SAFETY: advance returns a slice with the number of bytes we read, so, we return the only
        // byte in the slice.
        Ok(self.advance(std::mem::size_of::<u8>())[0])
    }
    /// Returns the value next byte. Function will fail if the length of the underlying slice is less
    /// than 1.
    pub fn peek_byte(&self, pos: usize) -> std::io::Result<u8> {
        self.check_available(std::mem::size_of::<u8>())?;
        // SAFETY: advance returns a slice with the number of bytes we read, so, we return the only
        // byte in the slice.
        Ok(self.peek_remaining()[pos])
    }
    /// Returns a reference to the next `n` bytes specified by the `len` parameter and advances the
    /// underlying slice by `len`. Function will fail if the length of the underlying slice is less
    /// than the size provided.
    pub fn read_bytes(&self, len: usize) -> std::io::Result<&'a [u8]> {
        self.check_and_advance(len)
    }
    /// Returns a reference to the next `n` bytes specified by the `len` parameter. Function will fail
    /// if the length of the underlying slice is less than the size provided.
    pub fn peek_bytes(&self, start: usize, len: usize) -> std::io::Result<&'a [u8]> {
        let end = start + len;
        self.check_available(end)?;
        Ok(&self.peek_remaining()[start..end])
    }
    /// Returns the length of the remaining buffer.
    pub fn len(&self) -> usize {
        self.buffer.get().len()
    }
    /// Returns the length of the remaining buffer.
    pub fn is_empty(&self) -> bool {
        self.buffer.get().is_empty()
    }
    /// Returns a reference to the remaining bytes in the slice.
    #[inline(always)]
    pub fn peek_remaining(&self) -> &'a [u8] {
        self.buffer.get()
    }
    /// Returns a reference to the remaining bytes in the slice.
    #[inline(always)]
    pub fn get_remaining(self) -> &'a [u8] {
        self.buffer.get()
    }
    /// Checks that there are enough bytes left in the slice to advance the start of the slice position,
    /// and returns a slice from the previous start of the buffer to the new start of the buffer.
    fn check_and_advance(&self, len: usize) -> std::io::Result<&'a [u8]> {
        self.check_available(len)?;
        Ok(self.advance(len))
    }
    pub fn find_bytes(&self, pat: &[u8]) -> Option<usize> {
        let buffer = self.buffer.get();
        let pat_len = pat.len();
        let mut i = 0;

        while i < buffer.len() - (pat_len - 1) {
            if &buffer[i..pat_len + i] == pat {
                return Some(i);
            }

            i += 1;
        }

        None
    }
    /// Advance the start of the buffer by the number of bytes provided by `len`. Returns a slice from
    /// the previous start of the buffer up until the new start of the buffer.
    ///
    /// # Safety
    ///
    /// Caller should call `self.check_available(size)` before calling this to check if there is room
    /// in the buffer to advance.
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

    #[test]
    fn peek() {
        let hello_world = b"Hello, World!";
        let br = BufferReader::new(hello_world);
        let len = br.len();
        let hello = std::str::from_utf8(br.peek_bytes(5, 2).unwrap()).unwrap();

        assert_eq!(len, br.len());
        assert_eq!(hello, ", ");
    }

    /// A test type to make sure read_t and peek_t work.
    #[repr(C, packed(1))]
    struct TestT {
        int_one: u32,
        byte: u8,
    }

    pub const TEST_T_SIZE: usize = 0x5;
    const _: () = assert!(std::mem::size_of::<TestT>() == TEST_T_SIZE);

    #[test]
    fn read_t() {
        let hello_world = b"Hello, World!";
        let br = BufferReader::new(hello_world);
        let test_t = br.read_t::<TestT>().unwrap();
        let int = test_t.int_one;
        assert_eq!(int, u32::from_le_bytes(*b"Hell"));
        assert_eq!(test_t.byte, b'o');
    }

    #[test]
    fn peek_t() {
        let hello_world = b"Hello, World!";
        let br = BufferReader::new(hello_world);
        let len = br.len();
        let test_t = br.peek_t::<TestT>(7).unwrap();

        let int = test_t.int_one;
        assert_eq!(int, u32::from_le_bytes(*b"Worl"));
        assert_eq!(test_t.byte, b'd');
    }


    #[test]
    fn read_byte() {
        let hello_world = b"Hello, World!";
        let br = BufferReader::new(hello_world);
        let first_byte = br.read_byte().unwrap();

        assert_eq!(first_byte, b'H');
    }

    #[test]
    fn peek_byte() {
        let hello_world = b"Hello, World!";
        let br = BufferReader::new(hello_world);
        let seventh_byte = br.peek_byte(7).unwrap();

        assert_eq!(seventh_byte, b'W');
    }


    #[test]
    fn find() {
        let hello_world = b"Hello, World!";
        let br = BufferReader::new(hello_world);
        let hello = br.find_bytes(b"o,").expect("Could not find pattern");

        assert_eq!(hello, 4);
    }

    #[test]
    fn find_end() {
        let hello_world = b"Hello, World!";
        let br = BufferReader::new(hello_world);
        let hello = br.find_bytes(b"d!").expect("Could not find pattern");

        assert_eq!(hello, 11);
    }

    #[test]
    #[should_panic]
    fn find_end_panic() {
        let hello_world = b"Hello, World!";
        let br = BufferReader::new(hello_world);
        let _ = br.find_bytes(b"! ").expect("Could not find pattern");
    }
}
