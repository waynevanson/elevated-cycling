use postcard::from_bytes_cobs;
use std::io::{BufRead, BufReader, Read};

fn read_cob_stream<T: serde::de::DeserializeOwned, R: Read>(
    reader: R,
) -> impl Iterator<Item = Result<T, postcard::Error>> {
    let mut buf_reader = BufReader::new(reader);

    std::iter::from_fn(move || {
        let mut buffer = Vec::new();
        let mut chunk = vec![0u8; 1024]; // Adjust the buffer size as needed

        // Read until a zero byte, which marks the end of a COB-encoded message
        match buf_reader.read_until(0, &mut chunk) {
            Ok(0) => None, // End of stream
            Ok(_) => {
                buffer.extend_from_slice(&chunk);
                from_bytes_cobs(buffer.as_mut_slice()).map(Some).transpose() // Decode COB and deserialize
            }
            Err(error) => Some(Err(postcard::Error::SerdeDeCustom)),
        }
    })
}
