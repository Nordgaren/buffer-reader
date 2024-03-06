use std::cell::Cell;
use std::io::{Error, ErrorKind, Read};
use bytemuck::AnyBitPattern;

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
    pub fn read_t<T: AnyBitPattern>(&self) -> std::io::Result<&'a T> {
        let size = std::mem::size_of::<T>();
        self.check_available(size)?;
        let slice = self.advance(size);
        // SAFETY: We know that the buffer passed back from `self.advance(size)?` is the size of T,
        // so we will assume that it's a valid T. This function is now considered safe, since we are
        // now requiring bytemuck and the `AnyBitPattern` trait.
        Ok(unsafe { &*(slice.as_ptr() as *const T) })
    }
    /// Returns a reference to the next `n` bytes in the slice as a reference to `T`, Where n is the
    /// size of `T`. Function will fail if there are not enough bytes left in the buffer.
    pub fn peek_t<T: AnyBitPattern>(&self, start: usize) -> std::io::Result<&'a T> {
        let end = start + std::mem::size_of::<T>();
        self.check_available(end)?;
        let slice = &self.peek_remaining()[start..end];
        // SAFETY: See read_t
        Ok(unsafe { &*(slice.as_ptr() as *const T) })
    }
    /// Returns a reference to the next `n` bytes in the slice as a reference to `T`. and then
    /// advances the slice by the size of `T` * `len` in bytes. Function will fail if the length of
    /// the underlying slice is less than the size of `T`.
    pub fn read_slice_t<T: AnyBitPattern>(&self, len: usize) -> std::io::Result<&'a [T]> {
        let size = len * std::mem::size_of::<T>();
        self.check_available(size)?;
        let slice = self.advance(size);
        // SAFETY: See read_t
        Ok(unsafe { core::slice::from_raw_parts(slice.as_ptr() as *const T, len) })
    }
    /// Returns a reference to the next `n` bytes in the slice as a reference to `T`, Where `n` is the
    /// size of `T` * `len`. Function will fail if there are not enough bytes left in the buffer.
    pub fn peek_slice_t<T: AnyBitPattern>(&self, start: usize, len: usize) -> std::io::Result<&'a [T]> {
        let end = start + (std::mem::size_of::<T>() * len);
        self.check_available(end)?;
        let slice = &self.peek_remaining()[start..end];
        // SAFETY: See read_t
        Ok(unsafe { core::slice::from_raw_parts(slice.as_ptr() as *const T, len) })
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
        // SAFETY: see read_byte
        Ok(self.peek_remaining()[pos])
    }
    /// Returns a reference to the next `n` bytes specified by the `len` parameter and advances the
    /// underlying slice by `len`. Function will fail if the length of the underlying slice is less
    /// than the size provided.
    pub fn read_bytes(&self, len: usize) -> std::io::Result<&'a [u8]> {
        self.check_and_advance(len)
        self.check_available(len)?;
        Ok(self.advance(len))
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
    /// Returns the position of the pattern of bytes provided, or `None` if the pattern is not found.
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
#[cfg(feature = "read")]
impl Read for BufferReader<'_> {
    /// # Warning - will copy bytes to provided buffer
    ///
    /// Pull some bytes from this source into the specified buffer, returning
    /// how many bytes were read.
    ///
    /// This function does not provide any guarantees about whether it blocks
    /// waiting for data, but if an object needs to block for a read and cannot,
    /// it will typically signal this via an [`Err`] return value.
    ///
    /// If the return value of this method is [`Ok(n)`], then implementations must
    /// guarantee that `0 <= n <= buf.len()`. A nonzero `n` value indicates
    /// that the buffer `buf` has been filled in with `n` bytes of data from this
    /// source. If `n` is `0`, then it can indicate one of two scenarios:
    ///
    /// 1. This reader has reached its "end of file" and will likely no longer
    ///    be able to produce bytes. Note that this does not mean that the
    ///    reader will *always* no longer be able to produce bytes. As an example,
    ///    on Linux, this method will call the `recv` syscall for a [`TcpStream`],
    ///    where returning zero indicates the connection was shut down correctly. While
    ///    for [`File`], it is possible to reach the end of file and get zero as result,
    ///    but if more data is appended to the file, future calls to `read` will return
    ///    more data.
    /// 2. The buffer specified was 0 bytes in length.
    ///
    /// It is not an error if the returned value `n` is smaller than the buffer size,
    /// even when the reader is not at the end of the stream yet.
    /// This may happen for example because fewer bytes are actually available right now
    /// (e. g. being close to end-of-file) or because read() was interrupted by a signal.
    ///
    /// As this trait is safe to implement, callers in unsafe code cannot rely on
    /// `n <= buf.len()` for safety.
    /// Extra care needs to be taken when `unsafe` functions are used to access the read bytes.
    /// Callers have to ensure that no unchecked out-of-bounds accesses are possible even if
    /// `n > buf.len()`.
    ///
    /// No guarantees are provided about the contents of `buf` when this
    /// function is called, so implementations cannot rely on any property of the
    /// contents of `buf` being true. It is recommended that *implementations*
    /// only write data to `buf` instead of reading its contents.
    ///
    /// Correspondingly, however, *callers* of this method in unsafe code must not assume
    /// any guarantees about how the implementation uses `buf`. The trait is safe to implement,
    /// so it is possible that the code that's supposed to write to the buffer might also read
    /// from it. It is your responsibility to make sure that `buf` is initialized
    /// before calling `read`. Calling `read` with an uninitialized `buf` (of the kind one
    /// obtains via [`MaybeUninit<T>`]) is not safe, and can lead to undefined behavior.
    ///
    /// [`MaybeUninit<T>`]: crate::mem::MaybeUninit
    ///
    /// # Errors
    ///
    /// If this function encounters any form of I/O or other error, an error
    /// variant will be returned. If an error is returned then it must be
    /// guaranteed that no bytes were read.
    ///
    /// An error of the [`ErrorKind::Interrupted`] kind is non-fatal and the read
    /// operation should be retried if there is nothing else to do.
    ///
    /// # Examples
    ///
    /// [`File`]s implement `Read`:
    ///
    /// [`Ok(n)`]: Ok
    /// [`File`]: crate::fs::File
    /// [`TcpStream`]: crate::net::TcpStream
    ///
    /// ```no_run
    /// use std::io;
    /// use std::io::prelude::*;
    /// use std::fs::File;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut f = File::open("foo.txt")?;
    ///     let mut buffer = [0; 10];
    ///
    ///     // read up to 10 bytes
    ///     let n = f.read(&mut buffer[..])?;
    ///
    ///     println!("The bytes: {:?}", &buffer[..n]);
    ///     Ok(())
    /// }
    /// ```
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.check_available(buf.len()) {
            Ok(_) => {
                buf.copy_from_slice(self.advance(buf.len()));
                Ok(buf.len())
            }
            Err(_) => {
                let len = self.len();
                buf[..len].copy_from_slice(self.advance(len));
                Ok(len)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "read")]
    fn read() {
        let hello_world = b"Hello, World!";
        let mut br = BufferReader::new(hello_world);

        let mut hello = [0; 5];
        let read = br.read(&mut hello[..]).unwrap();
        assert_eq!(read, 5);
        assert_eq!(&hello[..], b"Hello");

        let mut world = [0; 8];
        let read = br.read(&mut world[..]).unwrap();
        assert_eq!(read, 8);
        assert_eq!(&world[..], b", World!");

        // Check that the binary reader advanced through the entire buffer.
        assert_eq!(br.len(), 0);
    }

    #[test]
    fn read_bytes() {
        let hello_world = b"Hello, World!";
        let br = BufferReader::new(hello_world);

        let hello = br.read_bytes(5).unwrap();
        assert_eq!(&hello[..], b"Hello");

        // Check that the binary reader advanced through the "Hello".
        assert_eq!(br.len(), b", World!".len());
        let world = br.get_remaining();
        assert_eq!(&world[..], b", World!");
    }

    #[test]
    fn peek_bytes() {
        let hello_world = b"Hello, World!";
        let br = BufferReader::new(hello_world);
        let len = br.len();
        let hello = std::str::from_utf8(br.peek_bytes(5, 2).unwrap()).unwrap();

        assert_eq!(len, br.len());
        assert_eq!(hello, ", ");
    }

    /// A test type to make sure read_t and peek_t work.
    #[repr(C, packed(1))]
    #[derive(Copy, Clone, AnyBitPattern)]
    struct TestT {
        int_one: u32,
        byte: u8,
    }

    const TEST_T_SIZE: usize = std::mem::size_of::<u32>() + std::mem::size_of::<u8>();
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
