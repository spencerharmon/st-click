pub type BeatValue = f32;

pub const Semibreve: BeatValue = 4.0;
pub const WholeNote: BeatValue = 4.0;
pub const Minit: BeatValue = 2.0;
pub const HalfNote: BeatValue = 2.0;
pub const Crotchet: BeatValue = 1.0;
pub const QuarterNote: BeatValue = 1.0;
pub const Quaver: BeatValue = 0.5;
pub const EighthNote: BeatValue = 0.5;
pub const Semiquaver: BeatValue = 0.25;
pub const SixteenthNote: BeatValue = 0.25;

pub fn tuplet(note_type: BeatValue, n_tuplet: u16) -> BeatValue {
    let ret = note_type * 2.0 / n_tuplet as f32;
    println!("tuplet value: {:?}", ret);
    ret
}
