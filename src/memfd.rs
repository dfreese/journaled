const MEMFD_FILENAME: &std::ffi::CStr =
    unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"journald\0") };

#[derive(Debug)]
pub struct SealableFile {
    file: std::fs::File,
}

impl SealableFile {
    pub fn create() -> std::io::Result<Self> {
        use std::os::unix::prelude::FromRawFd;

        let fd = nix::sys::memfd::memfd_create(
            MEMFD_FILENAME,
            nix::sys::memfd::MemFdCreateFlag::MFD_ALLOW_SEALING,
        )
        .map_err(crate::helper::from_errno)?;

        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        Ok(Self { file })
    }

    pub fn seal(self) -> std::io::Result<SealedFile> {
        use std::os::unix::prelude::AsRawFd;

        let _ = nix::fcntl::fcntl(
            self.file.as_raw_fd(),
            nix::fcntl::FcntlArg::F_ADD_SEALS(nix::fcntl::SealFlag::all()),
        )
        .map_err(crate::helper::from_errno)?;

        Ok(SealedFile { file: self.file })
    }
}

impl std::io::Write for SealableFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        self.file.write_vectored(bufs)
    }
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.file.write_all(buf)
    }
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        self.file.write_fmt(fmt)
    }
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

#[derive(Debug)]
pub struct SealedFile {
    file: std::fs::File,
}

impl std::os::unix::io::AsRawFd for SealedFile {
    fn as_raw_fd(&self) -> i32 {
        self.file.as_raw_fd()
    }
}
