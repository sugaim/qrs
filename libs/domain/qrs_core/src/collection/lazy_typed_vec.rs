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

    fn try_from(value: LazyTypedVecBuffer) -> Result<Self, Self::Error> {
        if value.layout.size() == 0 || std::mem::size_of::<T>() == 0 {
            return Ok(Vec::new());
        }
        if std::mem::align_of::<T>() != value.layout.align() {
            return Err(anyhow!("Alignment mismatch"));
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
    #[inline]
    pub fn layout(&self) -> Layout {
        self.layout
    }

    #[inline]
    pub fn try_into_vec<T>(self) -> Result<Vec<T>, anyhow::Error> {
        self.try_into()
    }

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
