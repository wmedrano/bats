use std::fmt;

#[derive(Clone, PartialEq)]
pub struct Buffers {
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

impl Buffers {
    pub fn new(len: usize) -> Buffers {
        Buffers {
            left: vec![0.0; len],
            right: vec![0.0; len],
        }
    }
}

impl fmt::Debug for Buffers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_len = self.left.len().min(4);
        f.debug_struct("Buffers")
            .field("length", &self.left.len())
            .field("left", &&self.left[0..display_len])
            .field("right", &&self.right[0..display_len])
            .finish()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_buffers() {
        assert!(format!("{:?}", Buffers::new(1024)).len() < 1024);
        assert!(format!("{:?}", Buffers::new(1)).len() > 0);
    }
}
