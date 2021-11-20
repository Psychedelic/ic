use std::fs::File;
use std::io;
use std::path::Path;
use memmap::{Mmap, MmapOptions};

pub struct ScopedMmap {
    mmap: Option<Mmap>,
}

impl ScopedMmap {
    pub fn from_readonly_file(file: &File, len: usize) -> io::Result<Self> {
        if len == 0 {
            return Ok(ScopedMmap {
                mmap: None
            });
        }

        let mmap = unsafe { MmapOptions::new().len(len).map(file)? };
        Ok(ScopedMmap {
            mmap: Some(mmap)
        })
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let f = std::fs::File::open(path)?;
        let len = f.metadata()?.len() as usize;
        Self::from_readonly_file(&f, len)
    }

    /// Returns start address of the memory mapping.
    /// Prefer using `as_slice` whenever possible.
    pub fn addr(&self) -> *const u8 {
        if let Some(mmap) = &self.mmap {
            mmap.as_ptr()
        } else {
            std::ptr::null()
        }
    }

    /// Returns the length of the allocated region in bytes.
    pub fn len(&self) -> usize {
        self.mmap.as_ref().map_or(0, |m| m.len())
    }

    /// Returns true if the memory region has zero length.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a byte slice view of the memory mapping.
    pub fn as_slice(&self) -> &[u8] {
        self.as_ref()
    }
}

impl AsRef<[u8]> for ScopedMmap {
    fn as_ref(&self) -> &[u8] {
        if let Some(mmap) = &self.mmap {
            mmap.as_ref()
        } else {
            &[]
        }
    }
}
