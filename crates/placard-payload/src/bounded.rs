use flate2::{Decompress, FlushDecompress, Status};

use crate::MAX_DECODED_SIZE;

pub(crate) enum InflateError {
    Corrupt,
    TooLarge,
}

/// Decompresses a raw-DEFLATE stream into a buffer capped at
/// `MAX_DECODED_SIZE + 1` bytes, allocated up front. `Decompress::decompress`
/// with `FlushDecompress::Finish` never writes past the end of the output
/// slice it's given, so this bound holds regardless of what the compressed
/// input claims about its own decompressed size -- there's no unbounded
/// intermediate allocation for a malicious payload to exploit.
pub(crate) fn inflate_bounded(compressed: &[u8], dictionary: Option<&[u8]>) -> Result<Vec<u8>, InflateError> {
    let mut decompress = Decompress::new(false);
    if let Some(dictionary) = dictionary {
        decompress
            .set_dictionary(dictionary)
            .map_err(|_| InflateError::Corrupt)?;
    }

    let mut out = vec![0u8; MAX_DECODED_SIZE + 1];
    let status = decompress
        .decompress(compressed, &mut out, FlushDecompress::Finish)
        .map_err(|_| InflateError::Corrupt)?;

    let total_out = decompress.total_out() as usize;
    match status {
        Status::StreamEnd if total_out <= MAX_DECODED_SIZE => {
            out.truncate(total_out);
            Ok(out)
        }
        Status::StreamEnd | Status::BufError => Err(InflateError::TooLarge),
        Status::Ok => Err(InflateError::TooLarge),
    }
}
