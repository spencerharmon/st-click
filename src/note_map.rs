use wmidi;

fn midi_to_bytes(message: wmidi::MidiMessage<'_>) -> Vec<u8> {
    let mut bytes = vec![0u8; message.bytes_size()];
    message.copy_to_slice(bytes.as_mut_slice()).unwrap();
    bytes
}

pub fn cminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::CMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn dflatminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::DbMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn dminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::DMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn eflatminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::EbMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn eminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::EMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn fminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::FMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn gflatminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::GbMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn gminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::GMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn aflatminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::AbMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn aminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::AMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn bflatminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::BbMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn bminus1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::BMinus1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn c0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn dflat0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Db0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn d0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::D0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn eflat0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Eb0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn e0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::E0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn f0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::F0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn gflat0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Gb0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn g0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::G0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn aflat0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Ab0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn a0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn bflat0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Bb0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn b0_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::B0, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn c1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn dflat1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Db1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn d1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::D1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn eflat1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Eb1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn e1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::E1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn f1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::F1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn gflat1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Gb1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn g1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::G1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn aflat1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Ab1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn a1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn bflat1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Bb1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn b1_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::B1, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}
pub fn c2_on() -> Vec<u8> {
    let msg = wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C2, wmidi::U7::try_from(127).unwrap());
    midi_to_bytes(msg)
}

// pub fn c1_off(bytes: &mut [u8]) -> Option<&[u8]> {
//     note_off(Note::C1, bytes)
// }
