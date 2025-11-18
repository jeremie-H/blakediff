use std::path::Path;
use std::{
    fs::File,
    io::{self, Read},
};

/// Input abstraction for reading files
///
/// Automatically chooses between memory-mapped I/O and regular file I/O
/// based on file size and characteristics for optimal performance.
pub enum Input {
    /// Memory-mapped file input (used for files > 16KB)
    Mmap(io::Cursor<memmap2::Mmap>),
    /// Regular file input (used for small files)
    File(File),
}

impl Input {
    /// Open an input file, using mmap if appropriate
    ///
    /// Files larger than 16KB will be memory-mapped for better performance.
    /// Smaller files use regular file I/O to avoid mmap overhead.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to open
    ///
    /// # Returns
    ///
    /// * `Ok(Input)` - Successfully opened input
    /// * `Err(io::Error)` - Failed to open file
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        if let Some(mmap) = maybe_memmap_file(&file)? {
            return Ok(Self::Mmap(io::Cursor::new(mmap)));
        }
        Ok(Self::File(file))
    }

    /// Compute the BLAKE3 hash of the input
    ///
    /// Uses parallel hashing for memory-mapped files via Rayon.
    /// Regular files are hashed single-threaded with optimized buffer sizes.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Hexadecimal hash string
    /// * `Err(io::Error)` - I/O error during reading
    pub fn hash(&mut self) -> io::Result<String> {
        let mut hasher = blake3::Hasher::new();
        match self {
            // The fast path: If we mmapped the file successfully, hash using
            // multiple threads. This doesn't work on stdin, or on some files,
            // and it can also be disabled with --no-mmap.
            Self::Mmap(cursor) => {
                hasher.update_rayon(cursor.get_ref());
            }
            // The slower paths, for stdin or files we didn't/couldn't mmap.
            // This is currently all single-threaded. Doing multi-threaded
            // hashing without memory mapping is tricky, since all your worker
            // threads have to stop every time you refill the buffer, and that
            // ends up being a lot of overhead. To solve that, we need a more
            // complicated double-buffering strategy where a background thread
            // fills one buffer while the worker threads are hashing the other
            // one. We might implement that in the future, but since this is
            // the slow path anyway, it's not high priority.
            Self::File(file) => {
                copy_wide(file, &mut hasher)?;
            }
        }
        //Ok(hasher.finalize_xof())
        Ok(hasher.finalize().to_string())
    }
}

impl Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Mmap(cursor) => cursor.read(buf),
            Self::File(file) => file.read(buf),
        }
    }
}

// A 16 KiB buffer is enough to take advantage of all the SIMD instruction sets
// that we support, but `std::io::copy` currently uses 8 KiB. Most platforms
// can support at least 64 KiB, and there's some performance benefit to using
// bigger reads, so that's what we use here.
fn copy_wide(mut reader: impl Read, hasher: &mut blake3::Hasher) -> io::Result<u64> {
    let mut buffer = [0; 65536];
    let mut total = 0;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(total),
            Ok(n) => {
                hasher.update(&buffer[..n]);
                total += n as u64;
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
}

// Mmap a file, if it looks like a good idea. Return None in cases where we
// know mmap will fail, or if the file is short enough that mmapping isn't
// worth it. However, if we do try to mmap and it fails, return the error.
fn maybe_memmap_file(file: &File) -> io::Result<Option<memmap2::Mmap>> {
    let metadata = file.metadata()?;
    let file_size = metadata.len();
    Ok(
        if !metadata.is_file() ||// Not a real file.
            file_size > isize::MAX as u64 ||// Too long to safely map. https://github.com/danburkert/memmap-rs/issues/69
            file_size == 0 || // Mapping an empty file currently fails. https://github.com/danburkert/memmap-rs/issues/72
            file_size < 16 * 1024
        // Mapping small files is not worth it.
        {
            None
        } else {
            // Explicitly set the length of the memory map, so that filesystem
            // changes can't race to violate the invariants we just checked.
            let map = unsafe { memmap2::MmapOptions::new().len(file_size as usize).map(file)? };
            Some(map)
        },
    )
}
