use std::io;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;

/// The uid of the process on the other end of a connected Unix socket.
///
/// `getpeereid` reads the credentials the kernel recorded when the peer
/// connected, so a caller cannot state its own identity and cannot hand the
/// connection to another user afterwards.
pub fn peer_uid(stream: &UnixStream) -> io::Result<u32>
{
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } != 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(uid)
}

/// The socket file is already 0600, so this check is the second of two rather
/// than the only one. It is worth keeping: file permissions are a property of a
/// path, and a descriptor that outlives a mode change, a bind into a directory
/// that was briefly loose, or a future move of the socket all leave the path
/// check behind while this one still holds.
pub fn is_owner(stream: &UnixStream) -> io::Result<bool>
{
    Ok(peer_uid(stream)? == unsafe { libc::getuid() })
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn a_connection_from_this_user_is_recognised_as_its_own()
    {
        let (here, there) = UnixStream::pair().expect("a socket pair could not be made");
        assert_eq!(peer_uid(&here).expect("the peer uid should be readable"), unsafe { libc::getuid() });
        assert!(is_owner(&there).expect("the peer uid should be readable"));
    }
}
