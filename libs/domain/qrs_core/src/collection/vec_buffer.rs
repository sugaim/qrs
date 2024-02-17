// -----------------------------------------------------------------------------
// VecBuffer
//
/// A buffer for `Vec`.
///
/// This may be useful when
/// - you want to (re)use a `Vec` for various generic types without allocation for each type.
/// - you want a `Vec` struct field but do not want to declare a type parameter for it.
///
/// # Example
/// ```
/// use qrs_core::collection::VecBuffer;
///
/// let mut buf = VecBuffer::reuse(vec![0f64; 10]);
/// assert_eq!(buf.capacity(), 80); // 80 bytes
///
/// // use `buf` as a buffer for `Vec<i32>`
/// let ints: Vec<i32> = buf.into_vec();
/// assert_eq!(ints.capacity(), 20); // i32 has 4 bytes, so 20 elements are allowed
///
/// let mut buf = VecBuffer::reuse(ints);
/// assert_eq!(buf.capacity(), 80);
/// ```
#[derive(Debug)]
pub struct VecBuffer {
    data: Vec<u8>,
}

//
// construction
//
impl VecBuffer {
    /// Create an empty buffer
    #[inline]
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Create a new instance with an already allocated memory.
    pub fn reuse<T>(data: Vec<T>) -> Self {
        let unit_sz = std::mem::size_of::<T>();
        if unit_sz == 0 {
            return Self::new();
        }
        let ptr = data.as_ptr() as *const u8;
        let cap = data.capacity() * unit_sz;
        std::mem::forget(data); // to avoid freeing the memory via drop
        let data = unsafe { Vec::from_raw_parts(ptr as *mut u8, 0, cap) };
        Self { data }
    }
}

impl Default for VecBuffer {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<Vec<T>> for VecBuffer {
    #[inline]
    fn from(data: Vec<T>) -> Self {
        Self::reuse(data)
    }
}

impl<T> From<VecBuffer> for Vec<T> {
    #[inline]
    fn from(buf: VecBuffer) -> Self {
        buf.into_vec()
    }
}

//
// methods
//
impl VecBuffer {
    /// Get the current buffer size
    #[inline]
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Release buffered memory
    #[inline]
    pub fn release(&mut self) {
        self.data.clear();
        self.data.shrink_to_fit();
    }

    /// Convert the buffer into a concrete empty vector.
    pub fn into_vec<T>(mut self) -> Vec<T> {
        let unit_sz = std::mem::size_of::<T>();
        if unit_sz == 0 {
            return Vec::new();
        }

        let max_capacity = (self.data.capacity() / unit_sz) * unit_sz;
        self.data.shrink_to(max_capacity);
        let ptr = self.data.as_ptr() as *const T;
        let cap = self.data.capacity() / unit_sz;
        std::mem::forget(self.data); // to avoid freeing the memory via drop
        let data = unsafe { Vec::from_raw_parts(ptr as *mut T, 0, cap) };
        data
    }
}
