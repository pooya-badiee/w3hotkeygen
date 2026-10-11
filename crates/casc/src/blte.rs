//! BLTE, the container every stored file is wrapped in: one or more frames,
//! each plain (`N`) or zlib (`Z`). Encrypted (`E`) and nested (`F`) frames
//! aren't supported; the files this app reads use neither.

use crate::Error;

/// Bytes in front of the BLTE data inside a `data.###` archive entry: the
/// reversed encoded key (16), size (4), flags (2) and two checksums (8).
pub const LOCAL_HEADER: usize = 0x1e;

pub fn decode(data: &[u8]) -> Result<Vec<u8>, Error> {
    if data.get(..4) != Some(b"BLTE") {
        return Err(Error::Format("missing BLTE signature"));
    }
    let header_len = be32(data, 4)? as usize;
    if header_len == 0 {
        return frame(data.get(8..).unwrap_or_default());
    }
    // Flags (1), frame count (3), then per frame: encoded size, decoded size, MD5.
    let count = be32(data, 8)? as usize & 0xff_ffff;
    let mut out = Vec::new();
    let mut at = header_len;
    for i in 0..count {
        let encoded = be32(data, 12 + i * 24)? as usize;
        let body = data.get(at..at + encoded).ok_or(Error::Format("BLTE frame past the end"))?;
        out.extend(frame(body)?);
        at += encoded;
    }
    Ok(out)
}

fn frame(body: &[u8]) -> Result<Vec<u8>, Error> {
    let (&mode, rest) = body.split_first().ok_or(Error::Format("empty BLTE frame"))?;
    match mode {
        b'N' => Ok(rest.to_vec()),
        b'Z' => miniz_oxide::inflate::decompress_to_vec_zlib(rest).map_err(|_| Error::Format("bad zlib data")),
        b'E' => Err(Error::Unsupported("encrypted file")),
        _ => Err(Error::Unsupported("BLTE frame type")),
    }
}

fn be32(data: &[u8], at: usize) -> Result<u32, Error> {
    let bytes = data.get(at..at + 4).ok_or(Error::Format("BLTE header too short"))?;
    Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_plain_frame() {
        assert_eq!(decode(b"BLTE\0\0\0\0Nhello").unwrap(), b"hello");
    }

    #[test]
    fn framed_plain_and_zlib() {
        let zlib = miniz_oxide::deflate::compress_to_vec_zlib(b" world", 6);
        let mut data = b"BLTE".to_vec();
        let header_len = 12 + 2 * 24;
        data.extend((header_len as u32).to_be_bytes());
        data.extend([0x0f, 0, 0, 2]);
        for (enc, dec) in [(4, 3), (1 + zlib.len(), 6)] {
            data.extend((enc as u32).to_be_bytes());
            data.extend((dec as u32).to_be_bytes());
            data.extend([0; 16]);
        }
        data.extend(b"Nabc");
        data.push(b'Z');
        data.extend(&zlib);
        assert_eq!(decode(&data).unwrap(), b"abc world");
    }
}
