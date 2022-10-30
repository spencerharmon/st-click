use wmidi;

fn midi_to_bytes(message: wmidi::MidiMessage<'_>) -> Vec<u8> {
    let mut bytes = vec![0u8; message.bytes_size()];
    message.copy_to_slice(bytes.as_mut_slice()).unwrap();
    bytes
}

pub fn c1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}

// pub fn c1_off(bytes: &mut [u8]) -> Option<&[u8]> {
//     note_off(Note::C1, bytes)
// }
