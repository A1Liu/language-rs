use std::alloc::{alloc, dealloc, Layout};
use std::mem::size_of;

const BUCKET_SIZE: usize = 1024 * 1024;

#[derive(Clone, Copy)]
struct Bucket {
    begin: *mut u8,
    end: *mut u8,
}

struct Buckets {
    pub buckets: Vec<Bucket>,
}

impl Buckets {
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
        if self.buckets.len() == 0 {
            let begin = alloc(Layout::from_size_align_unchecked(BUCKET_SIZE, 8));
            self.buckets.push(Bucket { begin, end: begin });
        }

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

    pub fn new<T>(&mut self, t: T) -> &mut T {
        return unsafe { &mut *(self.new_unsafe(size_of::<T>()) as *mut T) };
    }

    pub fn store_array<T>(&mut self, values: Vec<T>) {
        let size = size_of::<T>();
        let mut location = unsafe { self.new_unsafe(values.len() * size) as *mut T };
        for value in values {
            unsafe {
                *location = value;
                location = location.add(1);
            }
        }
    }
}
