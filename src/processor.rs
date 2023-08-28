#[derive(Default, Debug)]
pub struct Processor {}

impl Processor {
    pub fn process<'a>(
        &mut self,
        _midi_in: impl Iterator<Item = (u32, &'a [u8])>,
        out_left: &mut [f32],
        out_right: &mut [f32],
    ) {
        clear(out_left);
        clear(out_right);
    }
}

/// Assign all values in `slice` to `0.0`.
fn clear(slice: &mut [f32]) {
    for v in slice.iter_mut() {
        *v = 0.0;
    }
}
