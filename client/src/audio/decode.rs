use opus::{Channels, Decoder};

pub fn decode_opus(encoded_data: &[u8]) -> Result<Vec<i16>, opus::Error> {
    let mut decoder = Decoder::new(48000, Channels::Stereo)?;

    let mut decoded_data = Vec::new();

    let len = decoder.decode(encoded_data, &mut decoded_data, false)?;

    decoded_data.truncate(len);

    Ok(decoded_data)
}
