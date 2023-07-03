use guile_3_sys::{
    scm_acons, scm_from_uint32, scm_from_uint64, scm_from_utf8_stringn, scm_length, scm_list_ref,
    scm_list_set_x, scm_make_list, scm_string_to_symbol, scm_symbol_to_string,
    scm_tc8_tags_scm_tc8_flag, scm_to_uint32, scm_to_uint64, scm_to_utf8_stringn, SCM,
};

/// A drop in replacement for SCM type with extra helpers.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Scm(SCM);

impl Scm {
    /// The emtpy list object: '().
    pub const EOL: Scm = Scm(scm_makiflag_bits(3) as SCM);
    /// The true value: #t.
    pub const TRUE: Scm = Scm(scm_makiflag_bits(4) as SCM);
    /// The false value: #f.
    pub const FALSE: Scm = Scm(scm_makiflag_bits(0) as SCM);
    /// The ELisp nil value.
    pub const ELISP_NIL: Scm = Scm(scm_makiflag_bits(1) as SCM);

    /// Returns the underlying `SCM` object.
    pub fn as_scm(self) -> SCM {
        self.0
    }

    /// Create a new `Scm` object with the `s` as a symbol.
    pub fn with_symbol(s: &str) -> Self {
        let raw = unsafe { scm_string_to_symbol(Into::<Scm>::into(s).as_scm()) };
        Scm::from(raw)
    }

    /// Create a list from `iter` where `iter`:
    ///   - Has a known size. Enforced by the `ExactSizeIterator` constraint.
    ///   - Iterates over elements that can be converted to `Scm` objects.
    pub fn from_exact_iter<T: Into<Scm>, I: ExactSizeIterator + Iterator<Item = T>>(
        iter: I,
    ) -> Scm {
        let len = Scm::from(iter.len() as u32);
        let list = Scm::from(unsafe { scm_make_list(len.into(), Scm::EOL.0) });
        for (idx, item) in iter.enumerate() {
            let scm_item = item.into();
            unsafe { scm_list_set_x(list.0, Scm::from(idx as u32).as_scm(), scm_item.as_scm()) };
        }
        list
    }

    /// Add a new key and value to an associated list. This function returns the new list and does
    /// not modify the original list. Equivalent to calling `(acons key value self)` in scheme.
    pub fn acons<K: Into<Scm>, V: Into<Scm>>(self, key: K, value: V) -> Scm {
        let alist = unsafe { scm_acons(key.into().0, value.into().0, self.0) };
        alist.into()
    }

    /// Return the `k`th element of the list. Equivalent to calling `(list-ref self k)` in Scheme.
    pub fn list_ref(self, k: usize) -> Scm {
        let v = unsafe { scm_list_ref(self.0, Scm::from(k as u32).into()) };
        Scm::from(v)
    }

    /// Get the length of the list. Equivalent to calling `(length self)` in Scheme.
    pub fn length(self) -> usize {
        let scm_len = Scm::from(unsafe { scm_length(self.0) });
        u64::from(scm_len) as usize
    }

    /// Convert a symbol to a string. Equivalent to calling `(symbol-to-string self)` in Scheme.
    pub fn symbol_to_str(self) -> Scm {
        let scm_str = unsafe { scm_symbol_to_string(self.0) };
        Scm::from(scm_str)
    }
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

impl From<SCM> for Scm {
    fn from(scm: SCM) -> Scm {
        Scm(scm)
    }
}

impl From<bool> for Scm {
    fn from(b: bool) -> Scm {
        if b {
            Scm::TRUE
        } else {
            Scm::FALSE
        }
    }
}

impl From<u32> for Scm {
    fn from(scm: u32) -> Scm {
        unsafe { scm_from_uint32(scm).into() }
    }
}

impl From<u64> for Scm {
    fn from(scm: u64) -> Scm {
        unsafe { scm_from_uint64(scm).into() }
    }
}

impl From<String> for Scm {
    fn from(s: String) -> Scm {
        Scm::from(s.as_str())
    }
}

impl From<&str> for Scm {
    fn from(s: &str) -> Scm {
        unsafe { scm_from_utf8_stringn(s.as_ptr() as _, s.len() as _).into() }
    }
}

impl<T: Into<Scm>> From<Option<T>> for Scm {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => v.into(),
            None => Scm::EOL,
        }
    }
}

impl From<Scm> for SCM {
    fn from(scm: Scm) -> Self {
        scm.0
    }
}

impl From<Scm> for u32 {
    fn from(scm: Scm) -> u32 {
        unsafe { scm_to_uint32(scm.0) }
    }
}

impl From<Scm> for u64 {
    fn from(scm: Scm) -> u64 {
        unsafe { scm_to_uint64(scm.0) }
    }
}

impl From<Scm> for String {
    fn from(scm: Scm) -> String {
        unsafe {
            let mut len = 0;
            let ptr = scm_to_utf8_stringn(scm.0, &mut len) as *mut u8;
            String::from_raw_parts(ptr, len as usize, len as usize)
        }
    }
}

impl From<Scm> for String {
    fn from(scm: Scm) -> String {
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
        unsafe { scm_nil_p(Scm::EOL.into()) };
    }
}
