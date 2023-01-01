# journaled
journaled is an API to interact with the systemd journal.  It's primarily
intended to be used by other implementations.  It does provide a `log::Log` and
a (poor) `slog::Drain` implementation as a point of reference.

Integration testing and documentation are still a work in progress.
