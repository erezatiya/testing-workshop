use std::str;

const NEWLINE: &[u8] = b"\r\n";

#[derive(Debug, PartialEq)]
enum RespError {
    MissingLength,
    InvalidLength,
    InvalidData,
    MissingEndOfLine,
    NotEnoughData {
        required_len: usize,
        actual_len: usize,
    },
}

fn resp_parse(data: &[u8]) -> Result<&[u8], RespError> {
    match &data {
        [b'+', data @ ..] => parse_simple_string(data),
        [b'$', data @ ..] => parse_bulk_string(data),
        _ => Err(RespError::InvalidData),
    }
}

fn parse_simple_string(data: &[u8]) -> Result<&[u8], RespError> {
    match split_line(data) {
        (Some(line), _) => Ok(line),
        (None, _) => Err(RespError::MissingEndOfLine),
    }
}

fn parse_bulk_string(data: &[u8]) -> Result<&[u8], RespError> {
    match split_line(data) {
        (Some(length), data) => {
            let length = str::from_utf8(length).map_err(|_| RespError::InvalidLength)?;
            let length: usize = length.parse().map_err(|_| RespError::InvalidLength)?;

            let required_len = length + NEWLINE.len();
            let actual_len = data.len();
            if actual_len < required_len {
                Err(RespError::NotEnoughData {
                    required_len,
                    actual_len,
                })
            } else {
                let data = &data[..length];
                Ok(data)
            }
        }
        (None, _) => Err(RespError::MissingLength),
    }
}

fn split_line(data: &[u8]) -> (Option<&[u8]>, &[u8]) {
    find_subsequence(data, NEWLINE)
        .map(|i| {
            let line = &data[..i];
            let rest = &data[i + NEWLINE.len()..];
            (Some(line), rest)
        })
        .unwrap_or((None, data))
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[test]
fn test_resp_parse_simple() {
    let table = &[
        (b"+hello\r\n".as_ref(), b"hello".as_ref()),
        (b"+hello\r\nfoo", b"hello"),
        (b"+hel\r\nlo\r\n", b"hel"),
        (b"+hel\rlo\r\n", b"hel\rlo"),
    ];

    for &(input, expected) in table {
        assert_parse_eq(input, expected);
    }
}

#[test]
fn test_resp_parse_bulk() {
    let table_good = &[
        (b"$11\r\nhello world\r\n".as_ref(), b"hello world".as_ref()),
        (b"$12\r\nhello\r\nworld\r\n", b"hello\r\nworld"),
        (b"$11\r\nhello\rworld\r\n", b"hello\rworld"),
    ];

    for (input, expected) in table_good {
        assert_parse_eq(input, expected);
    }

    let table_bad = &[
        (b"$".as_ref(), RespError::MissingLength),
        (b"$11", RespError::MissingLength),
        (b"", RespError::InvalidData),
        (b"ZZZZZZZ", RespError::InvalidData),
        (b"$11hello\r\n", RespError::InvalidLength),
        (
            b"$11\r\n",
            RespError::NotEnoughData {
                required_len: 11 + NEWLINE.len(),
                actual_len: 0,
            },
        ),
    ];

    for (input, expected_error) in table_bad {
        assert_parse_error(input, expected_error);
    }
}

fn assert_parse_eq(input: &[u8], expected: &[u8]) {
    let parsed = resp_parse(input).unwrap();
    assert_eq!(
        parsed,
        expected,
        "expected: '{}', got: '{}'",
        String::from_utf8_lossy(expected),
        String::from_utf8_lossy(parsed),
    );
}

fn assert_parse_error(input: &[u8], error: &RespError) {
    match resp_parse(input) {
        Err(ref e) => assert_eq!(e, error),
        r => panic!("got unexpected result: {:?}", r),
    }
}
