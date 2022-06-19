/// Describes state of a note
#[derive(Debug)]
pub struct Key {
    pub state: bool,
    pub keynumber: usize,
    pub sample_clock: f32,
}
