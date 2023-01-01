pub trait SendFd {
    fn send_fd_to<F, P>(&self, file: F, path: P) -> std::io::Result<usize>
    where
        F: std::os::unix::io::AsRawFd,
        P: AsRef<std::path::Path>;
}

impl SendFd for std::os::unix::net::UnixDatagram {
    fn send_fd_to<F, P>(&self, file: F, path: P) -> std::io::Result<usize>
    where
        F: std::os::unix::io::AsRawFd,
        P: AsRef<std::path::Path>,
    {
        use std::os::unix::io::AsRawFd;

        let fds = &[file.as_raw_fd()];
        let ancillary = [nix::sys::socket::ControlMessage::ScmRights(fds)];

        let path =
            nix::sys::socket::UnixAddr::new(path.as_ref()).map_err(crate::helper::from_errno)?;
        nix::sys::socket::sendmsg(
            self.as_raw_fd(),
            &[],
            &ancillary,
            nix::sys::socket::MsgFlags::empty(),
            Some(&path),
        )
        .map_err(crate::helper::from_errno)
    }
}
