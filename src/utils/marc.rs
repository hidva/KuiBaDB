// Copyright 2020 <盏一 w@hidva.com>
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use std::marker::PhantomData;
use std::ptr::{self, NonNull};
use std::sync::atomic::{self, AtomicUsize, Ordering, Ordering::Relaxed};

pub trait Destory {
    type DestoryCtx;
    fn destory(&mut self, ctx: &Self::DestoryCtx);
}

struct Inner<T: Destory> {
    rc: AtomicUsize,
    data: T,
}

pub struct Marc<T: Destory> {
    ptr: NonNull<Inner<T>>,
    phantom: PhantomData<Inner<T>>,
}

impl<T: Destory> Marc<T> {
    pub fn new(v: T) -> Self {
        let b = Box::new(Inner {
            rc: AtomicUsize::new(1),
            data: v,
        });
        Self::from_inner(Box::leak(b).into())
    }

    fn from_inner(ptr: NonNull<Inner<T>>) -> Self {
        Self {
            ptr,
            phantom: PhantomData,
        }
    }

    fn inner(&self) -> &Inner<T> {
        unsafe { self.ptr.as_ref() }
    }

    unsafe fn get_mut_unchecked(&mut self) -> &mut T {
        &mut (*self.ptr.as_ptr()).data
    }

    fn do_unref(&mut self, ctx: &T::DestoryCtx) {
        if self.inner().rc.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }

        atomic::fence(Ordering::Acquire);

        unsafe { self.get_mut_unchecked() }.destory(ctx);
        unsafe {
            Box::from_raw(self.ptr.as_ptr());
        }
        return;
    }

    // Arc::drop
    pub fn unref(mut self, ctx: &T::DestoryCtx) {
        self.do_unref(ctx);
        std::mem::forget(self);
        return;
    }
}

impl<T: Destory + Clone> Marc<T> {
    pub fn make_mut(&mut self, ctx: &T::DestoryCtx) -> &mut T {
        // As Arc said:
        // > Use Acquire to ensure that we see any writes to `weak`...
        // Since we have no weak refcount, we use Relaxed instead of Acquire here.
        if self
            .inner()
            .rc
            .compare_exchange(1, 0, Relaxed, Relaxed)
            .is_err()
        {
            let bak = Marc::new((**self).clone());
            self.do_unref(ctx);
            unsafe { ptr::write(self as *mut _, bak) };
        } else {
            self.inner().rc.store(1, Relaxed);
        }
        debug_assert_eq!(1, self.inner().rc.load(Relaxed));
        unsafe { self.get_mut_unchecked() }
    }
}

impl<T: Destory> Drop for Marc<T> {
    fn drop(&mut self) {
        // synchronizes with unref() according to
        // [Release-Acquire ordering](https://en.cppreference.com/w/cpp/atomic/memory_order#Release-Acquire_ordering)
        let rc = self.inner().rc.load(Ordering::Acquire);
        assert!(rc <= 1, "Marc::Drop: rc: {}", rc);

        unsafe {
            Box::from_raw(self.ptr.as_ptr());
        }
    }
}

const MAX_REFCOUNT: usize = (isize::MAX) as usize;

impl<T: Destory> std::clone::Clone for Marc<T> {
    fn clone(&self) -> Self {
        let rc = self.inner().rc.fetch_add(1, Ordering::Relaxed);
        assert!(rc <= MAX_REFCOUNT, "Marc::clone: rc: {}", rc);
        Self::from_inner(self.ptr)
    }
}

impl<T: Destory> std::ops::Deref for Marc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner().data
    }
}

#[cfg(test)]
mod test {
    use super::{Destory, Marc};
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn t() {
        #[derive(Default)]
        struct MarcTestCtx {
            destory_cnt: [AtomicU32; 8],
        }

        struct TestObj {
            id: usize,
        }

        impl Destory for TestObj {
            type DestoryCtx = MarcTestCtx;

            fn destory(&mut self, ctx: &Self::DestoryCtx) {
                ctx.destory_cnt[self.id].fetch_add(1, Ordering::Relaxed);
            }
        }

        let destory_ctx = MarcTestCtx::default();
        let marc1 = Marc::new(TestObj { id: 3 });
        {
            let marc2 = marc1.clone();
            marc2.unref(&destory_ctx);
        }
        let id = marc1.id;
        assert_eq!(0, destory_ctx.destory_cnt[id].load(Ordering::Relaxed));
        marc1.unref(&destory_ctx);
        assert_eq!(1, destory_ctx.destory_cnt[id].load(Ordering::Relaxed));

        let id2 = 7;
        {
            let _ = Marc::new(TestObj { id: id2 });
        }
        assert_eq!(0, destory_ctx.destory_cnt[id2].load(Ordering::Relaxed));
    }
}

unsafe impl<T: Sync + Send + Destory> Send for Marc<T> {}
unsafe impl<T: Sync + Send + Destory> Sync for Marc<T> {}
