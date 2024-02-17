use std::{
    alloc::Layout,
    fmt::Debug,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use anyhow::anyhow;

// -----------------------------------------------------------------------------
// LazyTypedVecBuffer
//

/// A buffer for lazyly typed vector.
///
/// This may useful when you want to avoid the cost of allocation and deallocation
/// but it is hard to hold vector inside a struct due to difficulty of determining type,
/// for example, the type is determined from other generic type.
///
/// Allocated memory is characterized by [`Layout`].
/// Especially, alignment is important because we cannot use an allocated memory with different alignment.
/// So although this buffer can reduce the cost of allocation and deallocation
/// when instantinated element type of [`Vec`] has the same alignment,
/// this buffer may be useless when the alignment is different frequently.
///
/// # Example
/// ```
/// use std::convert::TryInto;
/// use std::alloc::Layout;
/// use qrs_core::collection::LazyTypedVecBuffer;
///
/// let layout = Layout::from_size_align(80, 8).unwrap();
/// let mut buffer = LazyTypedVecBuffer::new(layout);
///
/// // ok: alignment of u64 is 8
/// let vec = buffer.try_into_vec::<u64>().unwrap();
/// assert_eq!(vec.capacity(), 10);
/// let mut buffer = LazyTypedVecBuffer::reuse(vec);
///
/// // ok: alignment of f64 is 8.
/// let vec = buffer.try_into_vec::<f64>().unwrap();
/// assert_eq!(vec.capacity(), 10);
/// let mut buffer = LazyTypedVecBuffer::reuse(vec);
///
/// // err: alignment of u32 is 4.
/// // in this case, the buffer is deallocated.
/// assert!(buffer.try_into_vec::<u32>().is_err());
/// ```
#[derive(Debug)]
pub struct LazyTypedVecBuffer {
    ptr: NonNull<u8>,
    layout: Layout,
}

impl Drop for LazyTypedVecBuffer {
    fn drop(&mut self) {
        if self.layout.size() != 0 {
            unsafe { std::alloc::dealloc(self.ptr.as_ptr(), self.layout) }
        }
    }
}

//
// construction
//
impl LazyTypedVecBuffer {
    #[inline]
    pub fn new(layout: Layout) -> Self {
        Self {
            ptr: if layout.size() == 0 {
                NonNull::dangling()
            } else {
                NonNull::new(unsafe { std::alloc::alloc(layout) }).unwrap()
            },
            layout,
        }
    }

    pub fn reuse<T>(v: Vec<T>) -> Self {
        if v.capacity() == 0 || std::mem::size_of::<T>() == 0 {
            return Self::new(Layout::from_size_align(0, std::mem::align_of::<T>()).unwrap());
        }
        let ptr = v.as_ptr() as *mut u8;
        let size = v.capacity() * std::mem::size_of::<T>();
        std::mem::forget(v); // manually take the ownership
        Self {
            ptr: NonNull::new(ptr).unwrap(),
            layout: Layout::from_size_align(size, std::mem::align_of::<T>()).unwrap(),
        }
    }
}

impl Default for LazyTypedVecBuffer {
    #[inline]
    fn default() -> Self {
        Self::new(Layout::from_size_align(0, 1).unwrap())
    }
}

impl<T> From<Vec<T>> for LazyTypedVecBuffer {
    #[inline]
    fn from(data: Vec<T>) -> Self {
        Self::reuse(data)
    }
}

impl<T> TryFrom<LazyTypedVecBuffer> for Vec<T> {
    type Error = anyhow::Error;

    /// Create an empty vector from the buffer.
    ///
    /// Returns an error if the alignment of the buffer is different from the requested type.
    /// When alignment matches, the ownership of memory held by the buffer
    /// is moved to the returned vector.
    fn try_from(value: LazyTypedVecBuffer) -> Result<Self, Self::Error> {
        if value.layout.size() == 0 || std::mem::size_of::<T>() == 0 {
            return Ok(Vec::new());
        }
        if std::mem::align_of::<T>() != value.layout.align() {
            return Err(anyhow!(
                "Alignment mismatch. Allocated memory assumes {} but requested {}",
                value.layout.align(),
                std::mem::align_of::<T>()
            ));
        }
        let cap = value.layout.size() / std::mem::size_of::<T>();
        let new_size = cap * std::mem::size_of::<T>();
        let ptr = unsafe { std::alloc::realloc(value.ptr.as_ptr(), value.layout, new_size) };
        std::mem::forget(value); // in this route, the ownership is taken by the returned Vec.
        Ok(unsafe { Vec::from_raw_parts(ptr as _, 0, cap) })
    }
}

//
// methods
//
impl LazyTypedVecBuffer {
    /// Get the layout of the buffer
    #[inline]
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// Try to convert the buffer to a vector of the requested type.
    ///
    /// Returns an error if the alignment of the buffer is different from the requested type.
    /// When alignment matches, the ownership of memory held by the buffer
    /// is moved to the returned vector.
    #[inline]
    pub fn try_into_vec<T>(self) -> Result<Vec<T>, anyhow::Error> {
        self.try_into()
    }

    /// Convert into an empty vector of the requested type.
    ///
    /// When generated RAII object is dropped, the ownership of vector
    /// is returned to the buffer.
    #[inline]
    pub fn as_vec_mut<T>(&mut self) -> LazyTypedVec<T> {
        let shallow_copy = Self {
            ptr: self.ptr,
            layout: self.layout,
        };
        self.ptr = NonNull::dangling();
        self.layout = Layout::from_size_align(0, self.layout.align()).unwrap();
        LazyTypedVec {
            buffer: self,
            vec: shallow_copy.try_into().unwrap_or_default(),
        }
    }
}

// -----------------------------------------------------------------------------
//  LazyTypedVec
//
pub struct LazyTypedVec<'a, T> {
    buffer: &'a mut LazyTypedVecBuffer,
    vec: Vec<T>,
}

impl<'a, T> Drop for LazyTypedVec<'a, T> {
    fn drop(&mut self) {
        *self.buffer = LazyTypedVecBuffer::reuse(std::mem::take(&mut self.vec));
    }
}

//
// display, serde
//
impl<'a, T: Debug> Debug for LazyTypedVec<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.vec, f)
    }
}

//
// methods
//
impl<'a, T> Deref for LazyTypedVec<'a, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}

impl<'a, T> DerefMut for LazyTypedVec<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(Layout::from_size_align(0, 1).unwrap())]
    #[case(Layout::from_size_align(100, 1).unwrap())]
    #[case(Layout::from_size_align(0, 4).unwrap())]
    #[case(Layout::from_size_align(80, 4).unwrap())]
    #[case(Layout::from_size_align(0, 8).unwrap())]
    #[case(Layout::from_size_align(80, 8).unwrap())]
    #[case(Layout::from_size_align(0, 16).unwrap())]
    #[case(Layout::from_size_align(80, 16).unwrap())]
    fn test_new(#[case] layout: Layout) {
        let buffer = LazyTypedVecBuffer::new(layout);
        assert_eq!(buffer.layout(), layout);
    }

    #[test]
    fn test_default() {
        let buffer = LazyTypedVecBuffer::default();
        assert_eq!(buffer.layout(), Layout::from_size_align(0, 1).unwrap());
    }

    #[rstest]
    fn test_reuse() {
        let vec = vec![1u8, 2u8, 3u8];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(3, std::mem::align_of::<u8>()).unwrap()
        );
        let vec = buffer.try_into_vec::<u8>().unwrap();
        assert_eq!(vec.capacity(), 3);
        assert_eq!(vec.len(), 0);

        let vec = vec![1u32, 2u32, 3u32];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(12, std::mem::align_of::<u32>()).unwrap()
        );
        let vec = buffer.try_into_vec::<u32>().unwrap();
        assert_eq!(vec.capacity(), 3);
        assert_eq!(vec.len(), 0);

        let vec = vec![1f64, 2f64, 3f64];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(24, std::mem::align_of::<f64>()).unwrap()
        );
        let vec = buffer.try_into_vec::<f64>().unwrap();
        assert_eq!(vec.capacity(), 3);
        assert_eq!(vec.len(), 0);

        let vec = vec!["hoge".to_string(), "fuga".to_string()];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        let unit_sz = std::mem::size_of::<String>();
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(2 * unit_sz, std::mem::align_of::<String>()).unwrap()
        );
        let vec = buffer.try_into_vec::<String>().unwrap();
        assert_eq!(vec.capacity(), 2);
        assert_eq!(vec.len(), 0);

        // zero size
        let vec = vec![(), (), ()];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(0, std::mem::align_of::<()>()).unwrap()
        );
        let vec = buffer.try_into_vec::<()>().unwrap();
        assert_eq!(vec.capacity(), usize::MAX);
        assert_eq!(vec.len(), 0);

        let vec: Vec<u128> = Vec::new();
        let buffer = LazyTypedVecBuffer::reuse(vec);
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(0, std::mem::align_of::<u128>()).unwrap()
        );
        let vec = buffer.try_into_vec::<u128>().unwrap();
        assert_eq!(vec.capacity(), 0);
        assert_eq!(vec.len(), 0);
    }

    #[rstest]
    #[case(Layout::from_size_align(0, 1).unwrap())]
    #[case(Layout::from_size_align(100, 1).unwrap())]
    #[case(Layout::from_size_align(0, 4).unwrap())]
    #[case(Layout::from_size_align(80, 4).unwrap())]
    #[case(Layout::from_size_align(0, 8).unwrap())]
    #[case(Layout::from_size_align(80, 8).unwrap())]
    #[case(Layout::from_size_align(0, 16).unwrap())]
    #[case(Layout::from_size_align(80, 16).unwrap())]
    fn test_try_into_vec(#[case] layout: Layout) {
        // layout 1: bool
        let buffer = LazyTypedVecBuffer::new(layout);
        let vec = buffer.try_into_vec::<bool>();
        assert_eq!(vec.is_ok(), layout.align() == std::mem::align_of::<bool>());
        if let Ok(vec) = vec {
            assert_eq!(vec.capacity(), layout.size());
        }

        // layout 1: u8
        let buffer = LazyTypedVecBuffer::new(layout);
        let vec = buffer.try_into_vec::<u8>();
        assert_eq!(vec.is_ok(), layout.align() == std::mem::align_of::<u8>());
        if let Ok(vec) = vec {
            assert_eq!(vec.capacity(), layout.size());
        }

        // layout 4: u32
        let buffer = LazyTypedVecBuffer::new(layout);
        let vec = buffer.try_into_vec::<u32>();
        assert_eq!(vec.is_ok(), layout.align() == std::mem::align_of::<u32>());
        if let Ok(vec) = vec {
            assert_eq!(vec.capacity(), layout.size() / 4);
        }

        // layout 8: u64
        let buffer = LazyTypedVecBuffer::new(layout);
        let vec = buffer.try_into_vec::<u64>();
        assert_eq!(vec.is_ok(), layout.align() == std::mem::align_of::<u64>());
        if let Ok(vec) = vec {
            assert_eq!(vec.capacity(), layout.size() / 8);
        }

        // layout 16: u128
        let buffer = LazyTypedVecBuffer::new(layout);
        let vec = buffer.try_into_vec::<u128>();
        assert_eq!(vec.is_ok(), layout.align() == std::mem::align_of::<u128>());
        if let Ok(vec) = vec {
            assert_eq!(vec.capacity(), layout.size() / 16);
        }
    }

    #[test]
    fn test_as_vec_mut() {
        let mut buffer = LazyTypedVecBuffer::default();
        let cap = {
            let mut vec = buffer.as_vec_mut::<u8>();
            assert_eq!(vec.capacity(), 0);
            assert_eq!(vec.len(), 0);
            vec.push(1);
            vec.push(2);
            vec.push(3);
            vec.capacity()
        };
        // buffer inherits the ownership of the allocated memory used by vec.
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(cap, std::mem::align_of::<u8>()).unwrap()
        );
        {
            let vec = buffer.as_vec_mut::<u8>();
            assert_eq!(vec.capacity(), cap);
            assert_eq!(vec.len(), 0);
        };

        let cap = {
            let mut vec = buffer.as_vec_mut::<u32>();
            // buffer is deallocated because the alignment is different.
            assert_eq!(vec.capacity(), 0);
            assert_eq!(vec.len(), 0);
            vec.push(1);
            vec.push(2);
            vec.push(3);
            vec.capacity()
        };
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(cap * 4, std::mem::align_of::<u32>()).unwrap()
        );
    }
}
