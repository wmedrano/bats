#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)] // Consider leaving this one on.

// TODO: Consider using a sys crate for ffi. One could be made at the moment, but it is preferable
// to find an existing well maintained one.
pub mod ffi;

use ffi::*;

macro_rules! scm_pack {
    ($x:expr) => {
        $x as SCM
    };
}

macro_rules! scm_make_itag8_bits {
    ($n:expr, $tag:expr) => {
        ($n << 8) + $tag
    };
}

macro_rules! scm_makiflag_bits {
    ($n:expr) => {
        scm_make_itag8_bits!($n, scm_tc8_tags_scm_tc8_flag)
    };
}

macro_rules! scm_unpack {
    // # if defined __DECC || defined __HP_cc
    ($x:expr) => {
        $x as scm_t_bits
    }; // Figure out what to do with the other path.
}

macro_rules! scm_matches_bits_in_common {
    ($x:expr, $a:expr, $b:expr) => {
        ((scm_unpack!($x) & !(scm_unpack!($a) ^ scm_unpack!($b)))
            == (scm_unpack!($a) & scm_unpack!($b)))
    };
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Scm(pub SCM);

impl Scm {
    /// Equivalent to `#t` in Scheme.
    pub const TRUE: Scm = Scm(scm_pack!(scm_makiflag_bits!(4)));

    /// Equivalent to `#f` in Scheme.
    pub const FALSE: Scm = Scm(scm_pack!(scm_makiflag_bits!(0)));

    /// Equivalent to ELisp Nil.
    pub const ELISP_NIL: Scm = Scm(scm_pack!(scm_makiflag_bits!(1)));

    /// Equivalent to `'()` in Scheme.
    pub const EOL: Scm = Scm(scm_pack!(scm_makiflag_bits!(3)));

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_bool(b: bool) -> Scm {
        if b {
            Scm::TRUE
        } else {
            Scm::FALSE
        }
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_i8(x: i8) -> Scm {
        scm_from_int8(x).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_u8(x: u8) -> Scm {
        scm_from_uint8(x).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_i16(x: i16) -> Scm {
        scm_from_int16(x).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_u16(x: u16) -> Scm {
        scm_from_uint16(x).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_i32(x: i32) -> Scm {
        scm_from_int32(x).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_u32(x: u32) -> Scm {
        scm_from_uint32(x).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_i64(x: i64) -> Scm {
        scm_from_int64(x).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_u64(x: u64) -> Scm {
        scm_from_uint64(x).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_f64(x: f64) -> Scm {
        scm_from_double(x).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_string(s: &str) -> Scm {
        scm_from_utf8_stringn(s.as_ptr() as _, s.len() as _).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_symbol(s: &str) -> Scm {
        scm_from_utf8_symboln(s.as_ptr() as _, s.len() as _).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_keyword(s: &str) -> Scm {
        // The Guile C API only provides a scm_from_utf8_keyword (NOT keywordn) so we have to
        // convert to a symbol. To work around this, we convert to a symbol. Note that Guile's
        // keyword implementation does the same thing anyways.
        scm_symbol_to_keyword(Self::new_symbol(s).0).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_pair(x: Scm, y: Scm) -> Scm {
        Self::new_cons(x, y)
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn new_cons(x: Scm, y: Scm) -> Scm {
        scm_cons(x.0, y.0).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn with_reversed_list<I: Iterator<Item = Scm>>(iter: I) -> Scm {
        let mut head = Scm::EOL;
        for x in iter {
            head = Self::new_cons(x, head);
        }
        head
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn with_alist<I: Iterator<Item = (Scm, Scm)>>(iter: I) -> Scm {
        Self::with_reversed_list(iter.map(|(x, y)| Self::new_pair(x, y)))
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_bool(&self) -> bool {
        let is_false = scm_matches_bits_in_common!(self.0, Scm::ELISP_NIL.0, Scm::FALSE.0);
        !is_false
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_i8(&self) -> i8 {
        scm_to_int8(self.0)
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_u8(&self) -> u8 {
        scm_to_uint8(self.0)
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_i16(&self) -> i16 {
        scm_to_int16(self.0)
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_u16(&self) -> u16 {
        scm_to_uint16(self.0)
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_i32(&self) -> i32 {
        scm_to_int32(self.0)
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_u32(&self) -> u32 {
        scm_to_uint32(self.0)
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_i64(&self) -> i64 {
        scm_to_int64(self.0)
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_u64(&self) -> u64 {
        scm_to_uint64(self.0)
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_f64(&self) -> f64 {
        scm_to_double(self.0)
    }

    /// # Safety
    /// Uses unsafe functions.
    #[allow(clippy::inherent_to_string)]
    pub unsafe fn to_string(&self) -> String {
        let mut length = 0;
        let ptr: *mut u8 = scm_to_utf8_stringn(self.0, &mut length).cast();
        String::from_raw_parts(ptr, length as _, length as _)
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_symbol(&self) -> String {
        let s: Scm = scm_symbol_to_string(self.0).into();
        s.to_string()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn to_keyword(&self) -> String {
        let s: Scm = scm_keyword_to_symbol(self.0).into();
        s.to_symbol()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn car(self) -> Scm {
        ffi::scm_car(self.0).into()
    }

    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn cdr(self) -> Scm {
        ffi::scm_cdr(self.0).into()
    }

    /// Get the length of the list. Equivalent to calling `(length self)` in Scheme.
    ///
    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn length(self) -> usize {
        let scm_len = Scm(unsafe { ffi::scm_length(self.0) });
        scm_len.to_u64() as usize
    }

    /// Iterate through elements if this is a list.
    ///
    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn iter(self) -> impl Iterator<Item = Scm> {
        let len = self.length();
        // TODO: Use a better algorithm. This may be O(n) for each access makind iteration O(n^2).
        (0..len).map(move |idx| self.list_ref(idx))
    }

    /// Return the `k`th element of the list. Equivalent to calling `(list-ref self k)` in Scheme.
    ///
    /// # Safety
    /// Uses unsafe functions.
    pub unsafe fn list_ref(self, k: usize) -> Scm {
        let v = unsafe { scm_list_ref(self.0, Scm::new_u32(k as u32).0) };
        Scm(v)
    }
}

impl From<SCM> for Scm {
    fn from(value: SCM) -> Self {
        Scm(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_car() {
        unsafe {
            let a = Scm::new_i32(1);
            let b = Scm::new_i32(2);
            let c = Scm::new_cons(a, b);
            assert_eq!(c.car().to_i32(), 1i32);
        }
    }

    #[test]
    fn test_cdr() {
        unsafe {
            let a = Scm::new_i32(1);
            let b = Scm::new_i32(2);
            let c = Scm::new_cons(a, b);
            assert_eq!(c.cdr().to_i32(), 2i32);
        }
    }
}
