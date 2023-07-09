use guile_3_sys::*;

/// A drop in replacement for c SCM type with extra helpers.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Scm(SCM);

macro_rules! scm_pack {
    ($x:expr) => {
        ($x as SCM)
    };
}

macro_rules! scm_unpack {
    ($x:expr) => {
        ($x as scm_t_bits)
    };
}

macro_rules! scm_make_itag8_bits {
    ($x:expr, $tag:expr) => {
        ($x << 8) + $tag
    };
}

macro_rules! scm_makiflag_bits {
    ($n:expr) => {
        scm_make_itag8_bits!($n, scm_tc8_tags_scm_tc8_flag)
    };
}

macro_rules! scm_matches_bits_in_common {
    ($x:expr, $a:expr, $b:expr) => {
        ((scm_unpack!($x) & !(scm_unpack!($a) ^ scm_unpack!($b)))
            == (scm_unpack!($a) & scm_unpack!($b)))
    };
}

macro_rules! scm_pack_pointer {
    ($x:expr) => {
        scm_pack!($x as scm_t_bits)
    };
}

macro_rules! scm_unpack_pointer {
    ($x:expr) => {
        scm_unpack!($x) as *mut scm_t_bits
    };
}

macro_rules! scm2ptr {
    ($x:expr) => {
        scm_unpack_pointer!($x) as *mut scm_t_cell
    };
}

macro_rules! scm_gc_set_cell_object {
    ($x:expr, $n:expr, $v:expr) => {
        (*(scm2ptr!($x) as *mut SCM).offset($n)) = ($v)
    };
}

macro_rules! scm_gc_set_cell_word {
    ($x:expr, $n:expr, $v:expr) => {
        scm_gc_set_cell_object!(($x), ($n), scm_pack!($v))
    };
}

macro_rules! scm_validate_pair {
    ($cell:expr, $expr:expr) => {
        // The debug version is:
        //     ((!scm_is_pair (cell) ? scm_error_pair_access (cell), 0 : 0), (expr))
        $expr
    };
}

macro_rules! scm_gc_cell_object {
    ($x:expr, $n:expr) => {
        (*(scm2ptr!($x) as *mut SCM).offset($n))
    };
}

macro_rules! scm_cell_object {
    ($x:expr, $n:expr) => {
        scm_gc_cell_object!($x, $n)
    };
}

macro_rules! scm_cell_object_0 {
    ($x:expr) => {
        scm_cell_object!($x, 0)
    };
}

macro_rules! scm_cell_object_1 {
    ($x:expr) => {
        scm_cell_object!($x, 1)
    };
}

macro_rules! scm_car {
    ($x:expr) => {
        scm_validate_pair!($x, scm_cell_object_0!($x))
    };
}

macro_rules! scm_cdr {
    ($x:expr) => {
        scm_validate_pair!($x, scm_cell_object_1!($x))
    };
}

unsafe fn scm_cons(car: scm_t_bits, cdr: scm_t_bits) -> SCM {
    let sz = std::mem::size_of::<scm_t_cell>();
    let cell: SCM = scm_pack_pointer!(scm_gc_malloc(sz as _, std::ptr::null_mut()));
    scm_gc_set_cell_word!(cell, 1, cdr);
    scm_gc_set_cell_word!(cell, 0, car);
    cell
}

// SCM_INLINE_IMPLEMENTATION SCM
// scm_cell (scm_t_bits car, scm_t_bits cdr)
// {
//   SCM cell = SCM_PACK_POINTER (SCM_GC_MALLOC (sizeof (scm_t_cell)));

//   /* Initialize the type slot last so that the cell is ignored by the GC
//      until it is completely initialized.  This is only relevant when the GC
//      can actually run during this code, which it can't since the GC only runs
//      when all other threads are stopped.  */
//   SCM_GC_SET_CELL_WORD (cell, 1, cdr);
//   SCM_GC_SET_CELL_WORD (cell, 0, car);

//   return cell;
// }

impl Scm {
    /// The ELisp nil value.
    pub const ELISP_NIL: Scm = Scm(scm_pack!(scm_makiflag_bits!(1)));

    /// The empty list object: '().
    pub const EOL: Scm = Scm(scm_pack!(scm_makiflag_bits!(3)));

    /// The true value: #t.
    pub const TRUE: Scm = Scm(scm_pack!(scm_makiflag_bits!(4)));

    /// The false value: #f.
    pub const FALSE: Scm = Scm(scm_pack!(scm_makiflag_bits!(0)));

    pub const UNSPECIFIED: Scm = Scm(scm_pack!(scm_makiflag_bits!(8)));
    pub const UNDEFINED: Scm = Scm(scm_pack!(scm_makiflag_bits!(9)));
    pub const EOF_VAL: Scm = Scm(scm_pack!(scm_makiflag_bits!(10)));

    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn new<T: ToScm>(obj: T) -> Scm {
        obj.to_scm()
    }

    /// Create a new `Scm` object with the `s` as a symbol.
    ///
    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn with_symbol(s: &str) -> Self {
        let raw = unsafe { scm_string_to_symbol(s.to_scm().raw()) };
        raw.to_scm()
    }

    /// Create a new `Scm` object with `s` as a keyword.
    ///
    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn with_keyword(s: &str) -> Self {
        let sym = Scm::with_symbol(s);
        let raw_keyword = unsafe { scm_symbol_to_keyword(sym.raw()) };
        raw_keyword.to_scm()
    }

    /// Returns the underlying `SCM` object.
    ///
    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn raw(self) -> SCM {
        self.0
    }

    /// Create a list from `iter` where `iter`:
    ///   - Has a known size. Enforced by the `ExactSizeIterator` constraint.
    ///   - Iterates over elements that can be converted to `Scm` objects.
    ///
    /// # TODO
    ///   - Drop the `ExactSizeIterator` requirement.
    ///
    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn with_list<T: ToScm, I: ExactSizeIterator + Iterator<Item = T>>(iter: I) -> Scm {
        let len = (iter.len() as u32).to_scm();
        let list = (unsafe { scm_make_list(len.raw(), Scm::EOL.raw()) }).to_scm();
        for (idx, item) in iter.enumerate() {
            let scm_item = item.to_scm();
            unsafe { scm_list_set_x(list.0, Scm::new(idx as u32).raw(), scm_item.raw()) };
        }
        list
    }

    /// Add a new key and value to an associated list. This function returns the new list and does
    /// not modify the original list. Equivalent to calling `(acons key value self)` in scheme.
    ///
    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn acons<K: ToScm, V: ToScm>(self, key: K, value: V) -> Scm {
        let alist = unsafe { scm_acons(key.to_scm().raw(), value.to_scm().raw(), self.raw()) };
        alist.to_scm()
    }

    /// Return a newly allocated pair whose car is x and whose cdr is self. The pair is guaranteed to
    /// be different (in the sense of eq?) from every previously existing object.
    ///
    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn cons<T: ToScm>(self, x: T) -> Scm {
        let x = x.to_scm();
        let res = scm_cons(scm_unpack!(x.raw()), scm_unpack!(self.raw()));
        res.to_scm()
    }

    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn car(self) -> Scm {
        scm_car!(self.raw()).to_scm()
    }

    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn cdr(self) -> Scm {
        scm_cdr!(self.raw()).to_scm()
    }

    /// Return the `k`th element of the list. Equivalent to calling `(list-ref self k)` in Scheme.
    ///
    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn list_ref(self, k: usize) -> Scm {
        let v = unsafe { scm_list_ref(self.0, Scm::new(k as u32).raw()) };
        Scm::new(v)
    }

    /// Get the length of the list. Equivalent to calling `(length self)` in Scheme.
    ///
    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn length(self) -> usize {
        let scm_len = Scm::new(unsafe { scm_length(self.raw()) });
        u64::from_scm(scm_len) as usize
    }

    pub unsafe fn iter_list(self) -> impl ExactSizeIterator + Iterator<Item = Scm> {
        let len = self.length();
        (0..len).map(move |idx| self.list_ref(idx))
    }

    /// Convert a symbol to a string. Equivalent to calling `(symbol-to-string self)` in Scheme.
    ///
    /// # Safety
    /// Uses unsafe `ToScm` functions.
    pub unsafe fn symbol_to_str(self) -> Scm {
        let scm_str = unsafe { scm_symbol_to_string(self.0) };
        Scm::new(scm_str)
    }
}

/// Convert objects from `Scm`.
pub trait ToScm {
    /// Convert a `self` to  `Scm`.
    ///
    /// # Safety
    /// Conversions may or may not be safe.
    unsafe fn to_scm(self) -> Scm;
}

impl Default for Scm {
    fn default() -> Self {
        Self::EOL
    }
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

impl FromScm for bool {
    fn from_scm(scm: Scm) -> bool {
        let is_false = unsafe {
            scm_matches_bits_in_common!(scm.raw(), Scm::ELISP_NIL.raw(), Scm::FALSE.raw())
        };
        !is_false
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

impl FromScm for f32 {
    fn from_scm(scm: Scm) -> f32 {
        f64::from_scm(scm) as f32
    }
}

impl FromScm for f64 {
    fn from_scm(scm: Scm) -> f64 {
        unsafe { scm_to_double(scm.0) }
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

impl<T> FromScm for Option<T>
where
    T: FromScm,
{
    fn from_scm(scm: Scm) -> Option<T> {
        if bool::from_scm(unsafe { Scm::new(scm_nil_p(scm.raw())) }) {
            None
        } else {
            Some(T::from_scm(scm))
        }
    }
}

#[cfg(test)]
mod tests {
    use guile_3_sys::scm_nil_p;

    use super::*;

    #[test]
    fn scm_equality() {
        unsafe {
            let got = scm_nil_p(Scm::EOL.raw()).to_scm();
            assert_eq!(got.raw(), Scm::TRUE.raw());
        }
    }

    #[test]
    fn scm_bool() {
        assert_eq!(bool::from_scm(Scm::TRUE), true);
        assert_eq!(bool::from_scm(Scm::FALSE), false);
        assert_eq!(bool::from_scm(unsafe { Scm::new(true) }), true);
        assert_eq!(bool::from_scm(unsafe { Scm::new(false) }), false);
    }
}
