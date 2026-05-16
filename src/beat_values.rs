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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_value_aliases_match() {
        assert_eq!(Semibreve, WholeNote);
        assert_eq!(Minit, HalfNote);
        assert_eq!(Crotchet, QuarterNote);
        assert_eq!(Quaver, EighthNote);
        assert_eq!(Semiquaver, SixteenthNote);
    }

    #[test]
    fn note_value_ratios() {
        assert_eq!(WholeNote, 4.0);
        assert_eq!(HalfNote, 2.0);
        assert_eq!(QuarterNote, 1.0);
        assert_eq!(EighthNote, 0.5);
        assert_eq!(SixteenthNote, 0.25);
    }

    #[test]
    fn tuplet_triplet_of_quarter_is_two_thirds() {
        // A quarter-note triplet (3 in the space of 2 quarters)
        // value = 1.0 * 2 / 3 = 0.6666...
        let v = tuplet(QuarterNote, 3);
        assert!((v - (2.0 / 3.0)).abs() < 1e-6);
    }

    #[test]
    fn tuplet_septuplet_of_quarter() {
        // 7 in the space of 2 quarters -> 2/7
        let v = tuplet(QuarterNote, 7);
        assert!((v - (2.0 / 7.0)).abs() < 1e-6);
    }

    #[test]
    fn tuplet_duplet_of_quarter_equals_whole_note_division() {
        // 2 in the space of 2 = 1.0 (no-op)
        assert_eq!(tuplet(QuarterNote, 2), 1.0);
    }
}
