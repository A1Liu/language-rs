use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;
use std::mem::size_of;
use std::slice::from_raw_parts_mut;

const BUCKET_SIZE: usize = 1024 * 1024;

#[derive(Clone, Copy)]
pub struct Bucket {
    begin: *mut u8,
    end: *mut u8,
}

pub struct Buckets<'a> {
    pub buckets: Vec<Bucket>,
    unused: PhantomData<&'a u8>,
}

impl<'a> Buckets<'a> {
    pub fn new() -> Self {
        let begin = unsafe { alloc(Layout::from_size_align_unchecked(BUCKET_SIZE, 8)) };
        return Buckets {
            buckets: vec![Bucket { begin, end: begin }],
            unused: PhantomData,
        };
    }

    pub fn drop(&mut self) {
        for bucket in &self.buckets {
            unsafe {
                let mut size = (bucket.end as usize) - (bucket.begin as usize);
                if size <= BUCKET_SIZE {
                    size = BUCKET_SIZE
                }

                dealloc(bucket.begin, Layout::from_size_align_unchecked(size, 8));
            }
        }
    }

    pub unsafe fn new_unsafe(&mut self, size: usize) -> *mut u8 {
        let size = (size - 1) / 16 * 16 + 16;
        if size > BUCKET_SIZE {
            let bucket = self.buckets.last().unwrap().clone();
            let begin = alloc(Layout::from_size_align_unchecked(size, 8));
            *self.buckets.last_mut().unwrap() = Bucket {
                begin,
                end: begin.add(size),
            };
            self.buckets.push(bucket);
            return begin;
        }

        let mut last_bucket = self.buckets.last_mut().unwrap();
        let space_left = BUCKET_SIZE - ((last_bucket.end as usize) - (last_bucket.begin as usize));
        if space_left < size {
            let begin = alloc(Layout::from_size_align_unchecked(BUCKET_SIZE, 8));
            self.buckets.push(Bucket { begin, end: begin });
            last_bucket = self.buckets.last_mut().unwrap();
        }

        let ret_location = last_bucket.end;
        last_bucket.end = last_bucket.end.add(size);
        return ret_location;
    }

    pub fn add<T>(&mut self, t: T) -> &'a mut T {
        return unsafe { &mut *(self.new_unsafe(size_of::<T>()) as *mut T) };
    }

    pub fn add_array<T>(&mut self, values: Vec<T>) -> &'a mut [T] {
        let size = size_of::<T>();
        let len = values.len();
        let begin = unsafe { self.new_unsafe(values.len() * size) as *mut T };
        let mut location = begin;
        for value in values {
            unsafe {
                *location = value;
                location = location.add(1);
            }
        }

        return unsafe { from_raw_parts_mut(begin, len) };
    }
}
