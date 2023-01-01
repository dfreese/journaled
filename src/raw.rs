const JOURNALD_PATH: &str = "/run/systemd/journal/socket";
const FIELD_LEN_MAX: usize = 64;

pub const MESSAGE: Field = Field::unchecked("MESSAGE");
pub const MESSAGE_ID: Field = Field::unchecked("MESSAGE_ID");
pub const PRIORITY: Field = Field::unchecked("PRIORITY");
pub const CODE_FILE: Field = Field::unchecked("CODE_FILE");
pub const CODE_LINE: Field = Field::unchecked("CODE_LINE");
pub const CODE_FUNC: Field = Field::unchecked("CODE_FUNC");
pub const ERRNO: Field = Field::unchecked("ERRNO");
pub const INVOCATION_ID: Field = Field::unchecked("INVOCATION_ID");
pub const USER_INVOCATION_ID: Field = Field::unchecked("USER_INVOCATION_ID");
pub const SYSLOG_FACILITY: Field = Field::unchecked("SYSLOG_FACILITY");
pub const SYSLOG_IDENTIFIER: Field = Field::unchecked("SYSLOG_IDENTIFIER");
pub const SYSLOG_PID: Field = Field::unchecked("SYSLOG_PID");
pub const SYSLOG_TIMESTAMP: Field = Field::unchecked("SYSLOG_TIMESTAMP");
pub const SYSLOG_RAW: Field = Field::unchecked("SYSLOG_RAW");
pub const DOCUMENTATION: Field = Field::unchecked("DOCUMENTATION");
pub const TID: Field = Field::unchecked("TID");
pub const UNIT: Field = Field::unchecked("UNIT");
pub const USER_UNIT: Field = Field::unchecked("USER_UNIT");
pub const COREDUMP_UNIT: Field = Field::unchecked("COREDUMP_UNIT");
pub const COREDUMP_USER_UNIT: Field = Field::unchecked("COREDUMP_USER_UNIT");
pub const OBJECT_PID: Field = Field::unchecked("OBJECT_PID");

fn is_valid_field(field: &str) -> bool {
    if field.len() > FIELD_LEN_MAX {
        return false;
    }

    if let Some((first, rest)) = field.as_bytes().split_first() {
        // The allowed characters are:
        // * A-Z (always)
        // * _ (reserved for the first character, allowed for the rest)
        // * 0-9 (not allowed for the first)
        first.is_ascii_uppercase()
            && rest
                .iter()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == &b'_')
    } else {
        // Empty fields aren't allowed
        false
    }
}

/// Field represents an borrowed value of an already validated string.  systemd places specific
/// requirements on characters that can be used in its key, value pairs (though they are not
/// required to be unique).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Field<'a> {
    inner: &'a str,
}

impl<'a> Field<'a> {
    /// The field value is checked by [[`is_valid_field`]] and a Field is returned if true.
    pub fn validate(inner: &'a str) -> Option<Self> {
        if is_valid_field(inner) {
            Some(Self { inner })
        } else {
            None
        }
    }

    /// Allows for the construction of a potentially invalid Field.
    ///
    /// Since [[`is_valid_field`]] cannot reasonably be const, this allows for the construction of
    /// known valid field names at compile time.  It's expected that the validity is confirmed in
    /// tests by [[`Field::validate_unchecked`]].
    pub const fn unchecked(inner: &'a str) -> Self {
        Self { inner }
    }

    /// Validates an object created using [[`Field::unchecked`]].
    ///
    /// Every unchecked field should have a corresponding test that calls this.
    #[cfg(test)]
    fn validate_unchecked(&self) -> bool {
        is_valid_field(self.inner)
    }

    /// Capacity required in bytes when serialized.
    fn required_capacity(&self) -> usize {
        self.inner.len()
    }
}

/// OwnedField represents an already validated string.  Similar to Field, its value already conforms
/// to the constraints needed by systemd.  This can be used when constructing a field value that may
/// not be known at compile time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedField {
    inner: String,
}

impl OwnedField {
    pub fn sanitize<S>(field: S) -> Option<Self>
    where
        S: AsRef<str>,
    {
        let inner: String = field
            .as_ref()
            .chars()
            .skip_while(|c| !c.is_ascii_alphabetic())
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .take(FIELD_LEN_MAX)
            .collect();

        if inner.is_empty() {
            None
        } else {
            Some(Self { inner })
        }
    }

    pub fn sanitize_with_prefix<S>(field: S, prefix: Field) -> Self
    where
        S: AsRef<str>,
    {
        let iter = field
            .as_ref()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .take(FIELD_LEN_MAX - prefix.inner.len());

        Self {
            inner: prefix.inner.chars().chain(iter).collect(),
        }
    }
}

impl<'a> std::convert::From<&'a OwnedField> for Field<'a> {
    fn from(field: &'a OwnedField) -> Field<'a> {
        Field {
            inner: &field.inner,
        }
    }
}

/// Required capacity in bytes when a string is serialized for the journal with a [[`Field`]].
fn required_capacity(value: impl AsRef<str>) -> usize {
    value.as_ref().len() // payload length
            + 2 // separator ('\n') and end new line
            + 8 // u64 encoded len
}

/// Required capacity in bytes when a string is serialized for the journal with a [[`Field`]].
fn encoded_len(value: impl AsRef<str>) -> [u8; 8] {
    (value.as_ref().len() as u64).to_le_bytes()
}

/// Priority is an enum for the syslog-style values used by the systemd journal.
pub enum Priority {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
}

impl Priority {
    const fn as_str(&self) -> &'static str {
        match &self {
            Priority::Emergency => "0",
            Priority::Alert => "1",
            Priority::Critical => "2",
            Priority::Error => "3",
            Priority::Warning => "4",
            Priority::Notice => "5",
            Priority::Info => "6",
            Priority::Debug => "7",
        }
    }

    /// Is used to construct a tuple to be passed into [[`JournalWriter`]].
    pub const fn as_value(&self) -> (crate::raw::Field<'static>, &'static str) {
        (crate::raw::PRIORITY, self.as_str())
    }
}

pub struct JournalWriter {
    socket: std::os::unix::net::UnixDatagram,
}

impl JournalWriter {
    pub fn new() -> std::io::Result<Self> {
        let socket = std::os::unix::net::UnixDatagram::unbound()?;
        Ok(Self { socket })
    }

    pub fn check(&self) -> std::io::Result<()> {
        self.send(([] as [(Field, &str); 0]).into_iter())
    }

    pub fn send<'a, I, V>(&self, values: I) -> std::io::Result<()>
    where
        I: Iterator<Item = (Field<'a>, V)> + Clone,
        V: AsRef<str>,
    {
        let data = {
            let mut data = Vec::<u8>::new();
            data.reserve_exact(
                values
                    .clone()
                    .map(|(field, value)| field.required_capacity() + required_capacity(value))
                    .sum::<usize>(),
            );
            for (ref field, ref value) in values {
                data.extend(field.inner.as_bytes());
                data.push(b'\n');
                data.extend(encoded_len(value));
                data.extend(value.as_ref().as_bytes());
                data.push(b'\n');
            }
            data
        };

        // Try sending directly via the socket first.  If that fails, due to the message being too
        // large, send using a sealed memfd.  The max size is system dependent, which we could try
        // to figure out and store.  In lieu of that, just always try the fast path first.
        self.socket
            .send_to(&data, JOURNALD_PATH)
            .or_else(|err| {
                if let Some(nix::errno::Errno::EMSGSIZE) =
                    err.raw_os_error().map(nix::errno::Errno::from_i32)
                {
                    self.send_by_memfd(&data)
                } else {
                    Err(err)
                }
            })
            .map(|_| ())
    }

    fn send_by_memfd(&self, data: &[u8]) -> std::io::Result<usize> {
        use crate::socket::SendFd;
        use std::io::Write;

        let mut file = crate::memfd::SealableFile::create()?;
        file.write_all(data)?;
        let sealed = file.seal()?;

        let _ = self.socket.send_fd_to(sealed, JOURNALD_PATH)?;

        Ok(data.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_validate() {
        assert!(Field::validate("test").is_none());
        assert!(Field::validate("_TEST").is_none());
        assert!(Field::validate("").is_none());
        assert!(Field::validate(
            "IS_TOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO_LONG"
        )
        .is_none());
        assert!(Field::validate("TE%ST").is_none());
        assert!(Field::validate("0TEST").is_none());

        assert_eq!(Field::validate("TEST").unwrap().inner, "TEST");
        assert_eq!(Field::validate("T_EST_").unwrap().inner, "T_EST_");
    }

    #[test]
    fn test_field_required_capacity() {
        let field = "TEST";
        assert_eq!(
            Field::validate(field).unwrap().required_capacity(),
            field.len()
        );
    }

    #[test]
    fn test_well_known_fields_are_valid() {
        assert!(MESSAGE.validate_unchecked());
        assert!(MESSAGE_ID.validate_unchecked());
        assert!(PRIORITY.validate_unchecked());
        assert!(CODE_FILE.validate_unchecked());
        assert!(CODE_LINE.validate_unchecked());
        assert!(CODE_FUNC.validate_unchecked());
        assert!(ERRNO.validate_unchecked());
        assert!(INVOCATION_ID.validate_unchecked());
        assert!(USER_INVOCATION_ID.validate_unchecked());
        assert!(SYSLOG_FACILITY.validate_unchecked());
        assert!(SYSLOG_IDENTIFIER.validate_unchecked());
        assert!(SYSLOG_PID.validate_unchecked());
        assert!(SYSLOG_TIMESTAMP.validate_unchecked());
        assert!(SYSLOG_RAW.validate_unchecked());
        assert!(DOCUMENTATION.validate_unchecked());
        assert!(TID.validate_unchecked());
        assert!(UNIT.validate_unchecked());
        assert!(USER_UNIT.validate_unchecked());
    }

    #[test]
    fn test_required_capacity() {
        assert_eq!(required_capacity(""), 10);
        assert_eq!(required_capacity("t\nest"), 15);
    }

    #[test]
    fn test_sanitize() {
        assert!(OwnedField::sanitize("_______").is_none());
        assert_eq!(OwnedField::sanitize("_test_123").unwrap().inner, "TEST_123");

        assert!(OwnedField::sanitize("😂😂😂😂").is_none());
        assert_eq!(OwnedField::sanitize("😂😂A😂_😂B").unwrap().inner, "A___B");

        assert_eq!(
            OwnedField::sanitize("a".repeat(FIELD_LEN_MAX + 1)).unwrap(),
            OwnedField::sanitize("a".repeat(FIELD_LEN_MAX)).unwrap(),
        );
    }

    #[test]
    fn test_sanitize_with_prefix() {
        let foo = Field::validate("FOO").unwrap();

        assert_eq!(OwnedField::sanitize_with_prefix("___", foo).inner, "FOO___");
        assert_eq!(OwnedField::sanitize_with_prefix("_a1", foo).inner, "FOO_A1");

        assert_eq!(OwnedField::sanitize_with_prefix("😂😂", foo).inner, "FOO__");

        assert_eq!(
            OwnedField::sanitize_with_prefix("a".repeat(FIELD_LEN_MAX + 1), foo),
            OwnedField::sanitize("FOO".to_owned() + &"a".repeat(FIELD_LEN_MAX - 3)).unwrap(),
        );
    }

    #[test]
    fn test_into_field() {
        assert_eq!(
            Field::validate("TEST_123").unwrap(),
            OwnedField::sanitize("_test_123").as_ref().unwrap().into(),
        );
    }
}
