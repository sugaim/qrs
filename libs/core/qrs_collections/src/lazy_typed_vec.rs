use std::{
    alloc::Layout,
    any::TypeId,
    fmt::Debug,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

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
#[derive(Debug)]
pub struct LazyTypedVecBuffer {
    ptr: NonNull<u8>,
    layout: Layout,
    state: Option<(TypeId, usize)>,
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
            state: None,
        }
    }

    pub fn reuse<T: 'static>(v: Vec<T>) -> Self {
        if v.capacity() == 0 || std::mem::size_of::<T>() == 0 {
            return Self {
                ptr: NonNull::dangling(),
                layout: Layout::from_size_align(0, std::mem::align_of::<T>()).unwrap(),
                state: Some((TypeId::of::<T>(), v.len())),
            };
        }
        let ptr = v.as_ptr() as *mut u8;
        let size = v.capacity() * std::mem::size_of::<T>();
        let len = v.len();
        std::mem::forget(v); // manually take the ownership
        Self {
            ptr: NonNull::new(ptr).unwrap(),
            layout: Layout::from_size_align(size, std::mem::align_of::<T>()).unwrap(),
            state: Some((TypeId::of::<T>(), len)),
        }
    }
}

impl Default for LazyTypedVecBuffer {
    #[inline]
    fn default() -> Self {
        Self::new(Layout::from_size_align(0, 1).unwrap())
    }
}

impl<T: 'static> From<Vec<T>> for LazyTypedVecBuffer {
    #[inline]
    fn from(data: Vec<T>) -> Self {
        Self::reuse(data)
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

    /// Get the type of the buffer.
    ///
    /// When buffer have not used as a vector yet, this returns `None`.
    #[inline]
    pub fn prev_type(&self) -> Option<TypeId> {
        self.state.as_ref().map(|(tp, _)| *tp)
    }

    /// Get the empty vector of the requested type.
    ///
    /// If the alignment of the requested type is match with the buffer,
    /// memory which this buffer holds is reused.
    pub fn into_empty_vec<T: 'static>(self) -> Vec<T> {
        if std::mem::align_of::<T>() != self.layout.align() {
            // alignment mismatch
            return Vec::new();
        }
        if self.layout.size() == 0
            || std::mem::size_of::<T>() == 0
            || self.layout.size() < std::mem::size_of::<T>()
        {
            return Vec::new();
        }
        let cap = self.layout.size() / std::mem::size_of::<T>();
        let new_size = cap * std::mem::size_of::<T>();
        let ptr = unsafe { std::alloc::realloc(self.ptr.as_ptr(), self.layout, new_size) };
        std::mem::forget(self); // in this route, the ownership is taken by the returned Vec.
        unsafe { Vec::from_raw_parts(ptr as _, 0, cap) }
    }

    /// Try to restore the original vector.
    ///
    /// This method is different from [`LazyTypedVecBuffer::into_empty_vec`]
    /// because this method tries to restore the elements of the vector.
    ///
    /// # Errors
    /// - When this buffer is constructed without type specification.
    /// - When this buffer is constructed with a [`Vec`] of different type.
    ///
    /// In error case, an empty vector is returned.
    ///
    /// Note that even in this case, the buffer may be reused by the returned vector.
    /// For example, when the alignment of the requested type is the same as the buffer
    /// even though the type is different.
    ///
    /// # Example
    /// ```
    /// use qrs_collections::LazyTypedVecBuffer;
    ///
    /// // reuse allocated memory(no allocation is performed in `try_restore` method)
    /// let buffer = LazyTypedVecBuffer::reuse(vec![1u64, 2u64, 3u64]);
    /// let vec = buffer.try_restore::<u64>();
    /// assert!(vec.is_ok());
    /// assert_eq!(vec.unwrap(), vec![1, 2, 3]);
    ///
    /// // Even when error case, the allocated memory can be reused
    /// // because the alignment matches.
    /// let layout = std::alloc::Layout::from_size_align(80, 8).unwrap();
    /// let buffer = LazyTypedVecBuffer::new(layout);
    /// let vec = buffer.try_restore::<u64>();
    /// assert!(vec.is_err());
    ///
    /// let vec = vec.unwrap_err();
    /// assert_eq!(vec.len(), 0);
    /// assert_eq!(vec.capacity(), 10);
    /// ```
    pub fn try_restore<T: 'static>(self) -> Result<Vec<T>, Vec<T>> {
        if std::mem::align_of::<T>() != self.layout.align() {
            // Can not use the buffer because the alignment is different.
            return Err(Vec::new());
        }
        if std::mem::size_of::<T>() == 0 {
            // we need a special treatment for zero size type
            // because nothing is allocated by [`Vec`].
            let mut res = Vec::new();
            return match self.state {
                Some((tp, len)) if tp == TypeId::of::<T>() => {
                    unsafe { res.set_len(len) };
                    Ok(res)
                }
                _ => Err(res),
            };
        }
        if self.layout.size() == 0 {
            // buffer is not allocated yet.
            return match self.state {
                Some((tp, _)) if tp == TypeId::of::<T>() => Ok(Vec::default()),
                _ => Err(Vec::new()),
            };
        }
        if self.layout.size() < std::mem::size_of::<T>() {
            // we can not reuse the buffer because the size is too small.
            // also, this if branch avoids zero size allocation.
            return Err(Vec::new());
        }

        // hereafter, the followings hold:
        // - alignment of T is the same as the buffer.
        // - size of T is not zero.
        // - buffer is allocated. especially, not the dangling pointer.
        // - buffer is large enough to hold at least one T.
        //
        // so allocated memory can be reused. (even though type is different)
        // if previous type is the same as T, we can restore the vector.
        // otherwise, we can reallocate the memory to the requested type.
        let cap = self.layout.size() / std::mem::size_of::<T>();
        let new_size = cap * std::mem::size_of::<T>();
        let ptr = unsafe { std::alloc::realloc(self.ptr.as_ptr(), self.layout, new_size) };
        assert!(!ptr.is_null(), "realloc failed");
        let len = match self.state {
            Some((tp, len)) if tp == TypeId::of::<T>() => Some(len),
            _ => None,
        };
        std::mem::forget(self); // in this route, the ownership is taken by the returned Vec.
        match len {
            Some(len) => unsafe { Ok(Vec::from_raw_parts(ptr as _, len, cap)) },
            None => unsafe { Err(Vec::from_raw_parts(ptr as _, 0, cap)) },
        }
    }

    /// Convert into an empty vector of the requested type.
    ///
    /// When generated RAII object is dropped, the ownership of vector
    /// is returned to the buffer.
    #[inline]
    pub fn as_empty_vec<T: 'static>(&mut self) -> LazyTypedVec<T> {
        let shallow_copy = Self {
            ptr: self.ptr,
            layout: self.layout,
            state: self.state,
        };
        self.ptr = NonNull::dangling();
        self.layout = Layout::from_size_align(0, self.layout.align()).unwrap();
        LazyTypedVec {
            buffer: self,
            vec: shallow_copy.into_empty_vec(),
        }
    }

    #[inline]
    pub fn as_restored_vec<T: 'static>(&mut self) -> Result<LazyTypedVec<T>, LazyTypedVec<T>> {
        let shallow_copy = Self {
            ptr: self.ptr,
            layout: self.layout,
            state: self.state,
        };
        self.ptr = NonNull::dangling();
        self.layout = Layout::from_size_align(0, self.layout.align()).unwrap();
        match shallow_copy.try_restore() {
            Ok(vec) => Ok(LazyTypedVec { buffer: self, vec }),
            Err(vec) => Err(LazyTypedVec { buffer: self, vec }),
        }
    }

    #[inline]
    pub fn free(&mut self) {
        if self.layout.size() != 0 {
            unsafe { std::alloc::dealloc(self.ptr.as_ptr(), self.layout) }
        }
        self.ptr = NonNull::dangling();
        self.layout = Layout::from_size_align(0, self.layout.align()).unwrap();
    }
}

// -----------------------------------------------------------------------------
//  LazyTypedVec
//
pub struct LazyTypedVec<'a, T: 'static> {
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
        let vec = buffer.into_empty_vec::<u8>();
        assert_eq!(vec.capacity(), 3);
        assert_eq!(vec.len(), 0);

        let vec = vec![1u32, 2u32, 3u32];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(12, std::mem::align_of::<u32>()).unwrap()
        );
        let vec = buffer.into_empty_vec::<u32>();
        assert_eq!(vec.capacity(), 3);
        assert_eq!(vec.len(), 0);

        let vec = vec![1f64, 2f64, 3f64];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(24, std::mem::align_of::<f64>()).unwrap()
        );
        let vec = buffer.into_empty_vec::<f64>();
        assert_eq!(vec.capacity(), 3);
        assert_eq!(vec.len(), 0);

        let vec = vec!["hoge".to_string(), "fuga".to_string()];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        let unit_sz = std::mem::size_of::<String>();
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(2 * unit_sz, std::mem::align_of::<String>()).unwrap()
        );
        let vec = buffer.into_empty_vec::<String>();
        assert_eq!(vec.capacity(), 2);
        assert_eq!(vec.len(), 0);

        // zero size
        let vec = vec![(), (), ()];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(0, std::mem::align_of::<()>()).unwrap()
        );
        let vec = buffer.into_empty_vec::<()>();
        assert_eq!(vec.capacity(), usize::MAX);
        assert_eq!(vec.len(), 0);

        let vec: Vec<u128> = Vec::new();
        let buffer = LazyTypedVecBuffer::reuse(vec);
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(0, std::mem::align_of::<u128>()).unwrap()
        );
        let vec = buffer.into_empty_vec::<u128>();
        assert_eq!(vec.capacity(), 0);
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn test_try_restore() {
        let vec = vec![1u8, 2u8, 3u8];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        let vec = buffer.try_restore::<u8>();
        assert!(vec.is_ok());
        assert_eq!(vec.unwrap(), vec![1, 2, 3]);

        let vec = vec![1u32, 2u32, 3u32];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        let vec = buffer.try_restore::<u32>();
        assert!(vec.is_ok());
        assert_eq!(vec.unwrap(), vec![1, 2, 3]);

        let vec = vec![1f64, 2f64, 3f64];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        let vec = buffer.try_restore::<f64>();
        assert!(vec.is_ok());
        assert_eq!(vec.unwrap(), vec![1.0, 2.0, 3.0]);

        let vec = vec!["hoge".to_string(), "fuga".to_string()];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        let vec = buffer.try_restore::<String>();
        assert!(vec.is_ok());
        assert_eq!(vec.unwrap(), vec!["hoge".to_string(), "fuga".to_string()]);

        // zero size
        let vec = vec![(), (), ()];
        let buffer = LazyTypedVecBuffer::reuse(vec);
        let vec = buffer.try_restore::<()>();
        assert!(vec.is_ok());
        assert_eq!(vec.unwrap(), vec![(), (), ()]);

        let vec: Vec<u128> = Vec::new();
        let buffer = LazyTypedVecBuffer::reuse(vec);
        let vec = buffer.try_restore::<u128>();
        assert!(vec.is_ok());
        assert_eq!(vec.unwrap(), vec![]);

        // error cases
        let mut vec = vec![1u8, 2u8, 3u8];
        vec.shrink_to_fit();
        let buffer = LazyTypedVecBuffer::reuse(vec);
        let vec = buffer.try_restore::<u32>();
        assert!(vec.is_err());
        let vec = vec.unwrap_err();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 0);

        let mut vec = vec![1u32, 2u32, 3u32];
        vec.shrink_to_fit();
        let buffer = LazyTypedVecBuffer::reuse(vec);
        let vec = buffer.try_restore::<i32>();
        assert!(vec.is_err());
        let vec = vec.unwrap_err();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 3);
    }

    #[test]
    fn test_as_empty_vec() {
        let mut buffer = LazyTypedVecBuffer::default();
        let cap = {
            let mut vec = buffer.as_empty_vec::<u8>();
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
            let vec = buffer.as_empty_vec::<u8>();
            assert_eq!(vec.capacity(), cap);
            assert_eq!(vec.len(), 0);
        };

        let cap = {
            let mut vec = buffer.as_empty_vec::<u32>();
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

    #[test]
    fn test_as_restored_vec() {
        let mut buffer = LazyTypedVecBuffer::default();
        let cap = {
            let vec = buffer.as_restored_vec::<u8>();
            assert!(vec.is_err());
            let mut vec = vec.unwrap_err();
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
            let vec = buffer.as_restored_vec::<u8>();
            assert!(vec.is_ok());
            let vec = vec.unwrap();
            assert_eq!(vec.capacity(), cap);
            assert_eq!(vec.len(), 3);
            assert_eq!(vec.deref(), &vec![1, 2, 3]);
        };

        let cap = {
            let mut vec = buffer.as_empty_vec::<u32>();
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
        {
            let vec = buffer.as_restored_vec::<u32>();
            assert!(vec.is_ok());
            let mut vec = vec.unwrap();
            assert_eq!(vec.capacity(), cap);
            assert_eq!(vec.len(), 3);
            assert_eq!(vec.deref(), &vec![1, 2, 3]);
            vec.shrink_to_fit();
        };
        {
            let vec = buffer.as_restored_vec::<i32>();
            assert!(vec.is_err());
            let vec = vec.unwrap_err();
            assert_eq!(vec.capacity(), 3);
            assert_eq!(vec.len(), 0);
        };
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
    fn test_free(#[case] layout: Layout) {
        let mut buffer = LazyTypedVecBuffer::new(layout);
        buffer.free();
        assert_eq!(
            buffer.layout(),
            Layout::from_size_align(0, layout.align()).unwrap()
        );
    }
}
