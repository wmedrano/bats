use guile_3_sys::*;

/// A drop in replacement for c SCM type with extra helpers.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Scm(SCM);

impl Scm {
    /// The empty list object: '().
    pub const EOL: Scm = Scm(scm_makiflag_bits(3) as SCM);
    /// The true value: #t.
    pub const TRUE: Scm = Scm(scm_makiflag_bits(4) as SCM);
    /// The false value: #f.
    pub const FALSE: Scm = Scm(scm_makiflag_bits(0) as SCM);
    /// The ELisp nil value.
    pub const ELISP_NIL: Scm = Scm(scm_makiflag_bits(1) as SCM);

    pub unsafe fn new<T: ToScm>(obj: T) -> Scm {
        obj.to_scm()
    }

    /// Create a new `Scm` object with the `s` as a symbol.
    pub unsafe fn with_symbol(s: &str) -> Self {
        let raw = unsafe { scm_string_to_symbol(s.to_scm().raw()) };
        raw.to_scm()
    }

    /// Returns the underlying `SCM` object.
    pub unsafe fn raw(self) -> SCM {
        self.0
    }

    /// Create a list from `iter` where `iter`:
    ///   - Has a known size. Enforced by the `ExactSizeIterator` constraint.
    ///   - Iterates over elements that can be converted to `Scm` objects.
    pub unsafe fn from_exact_iter<T: ToScm, I: ExactSizeIterator + Iterator<Item = T>>(
        iter: I,
    ) -> Scm {
        let len = (iter.len() as u32).to_scm();
        let list = (unsafe { scm_make_list(len.raw(), Scm::EOL.0) }).to_scm();
        for (idx, item) in iter.enumerate() {
            let scm_item = item.to_scm();
            unsafe { scm_list_set_x(list.0, Scm::new(idx as u32).raw(), scm_item.raw()) };
        }
        list
    }

    /// Add a new key and value to an associated list. This function returns the new list and does
    /// not modify the original list. Equivalent to calling `(acons key value self)` in scheme.
    pub unsafe fn acons<K: ToScm, V: ToScm>(self, key: K, value: V) -> Scm {
        let alist = unsafe { scm_acons(key.to_scm().0, value.to_scm().0, self.raw()) };
        alist.to_scm()
    }

    /// Return the `k`th element of the list. Equivalent to calling `(list-ref self k)` in Scheme.
    pub unsafe fn list_ref(self, k: usize) -> Scm {
        let v = unsafe { scm_list_ref(self.0, Scm::new(k as u32).raw()) };
        Scm::new(v)
    }

    /// Get the length of the list. Equivalent to calling `(length self)` in Scheme.
    pub unsafe fn length(self) -> usize {
        let scm_len = Scm::new(unsafe { scm_length(self.raw()) });
        u64::from_scm(scm_len) as usize
    }

    /// Convert a symbol to a string. Equivalent to calling `(symbol-to-string self)` in Scheme.
    pub unsafe fn symbol_to_str(self) -> Scm {
        let scm_str = unsafe { scm_symbol_to_string(self.0) };
        Scm::new(scm_str)
    }
}

/// Convert objects from `Scm`.
pub trait ToScm {
    /// Convert a `self` to  `Scm`.
    unsafe fn to_scm(self) -> Scm;
}

impl Default for Scm {
    fn default() -> Self {
        Self::EOL
    }
}

const fn scm_make_itag8_bits(x: u32, tag: u32) -> u32 {
    (x << 8) + tag
}

const fn scm_makiflag_bits(n: u32) -> u32 {
    scm_make_itag8_bits(n, scm_tc8_tags_scm_tc8_flag)
}

impl ToScm for Scm {
    unsafe fn to_scm(self) -> Scm {
        self
    }
}

impl ToScm for SCM {
    unsafe fn to_scm(self) -> Scm {
        Scm(self)
    }
}

impl ToScm for bool {
    unsafe fn to_scm(self) -> Scm {
        if self {
            Scm::TRUE
        } else {
            Scm::FALSE
        }
    }
}

impl ToScm for u8 {
    unsafe fn to_scm(self) -> Scm {
        unsafe { scm_from_uint8(self).to_scm() }
    }
}

impl ToScm for u32 {
    unsafe fn to_scm(self) -> Scm {
        unsafe { scm_from_uint32(self).to_scm() }
    }
}

impl ToScm for u64 {
    unsafe fn to_scm(self) -> Scm {
        unsafe { scm_from_uint64(self).to_scm() }
    }
}

impl ToScm for f32 {
    unsafe fn to_scm(self) -> Scm {
        (self as f64).to_scm()
    }
}

impl ToScm for f64 {
    unsafe fn to_scm(self) -> Scm {
        unsafe { scm_from_double(self).to_scm() }
    }
}

impl ToScm for &str {
    unsafe fn to_scm(self) -> Scm {
        unsafe { scm_from_utf8_stringn(self.as_ptr() as _, self.len() as _).to_scm() }
    }
}

impl ToScm for String {
    unsafe fn to_scm(self) -> Scm {
        self.as_str().to_scm()
    }
}

impl<T: ToScm> ToScm for Option<T> {
    unsafe fn to_scm(self) -> Scm {
        match self {
            Some(v) => v.to_scm(),
            None => Scm::EOL,
        }
    }
}

/// Convert from `Scm` objects.
pub trait FromScm {
    /// Convert a `scm` to  `Self`.
    fn from_scm(scm: Scm) -> Self;
}

impl FromScm for Scm {
    fn from_scm(scm: Scm) -> Self {
        scm
    }
}

impl FromScm for SCM {
    fn from_scm(scm: Scm) -> Self {
        scm.0
    }
}

impl FromScm for u32 {
    fn from_scm(scm: Scm) -> u32 {
        unsafe { scm_to_uint32(scm.0) }
    }
}

impl FromScm for u64 {
    fn from_scm(scm: Scm) -> u64 {
        unsafe { scm_to_uint64(scm.0) }
    }
}

impl FromScm for String {
    fn from_scm(scm: Scm) -> String {
        unsafe {
            let mut len = 0;
            let ptr = scm_to_utf8_stringn(scm.0, &mut len) as *mut u8;
            String::from_raw_parts(ptr, len as usize, len as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use guile_3_sys::scm_nil_p;

    use super::*;

    #[test]
    fn scm_eol_is_nil() {
        unsafe {
            let got = unsafe { scm_nil_p(Scm::EOL.raw()).to_scm() };
            assert_eq!(got.raw(), Scm::TRUE.raw());
        }
    }
}
