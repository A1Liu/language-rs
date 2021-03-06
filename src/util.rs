use std::alloc::{alloc, dealloc, Layout};
use std::char;
use std::collections::VecDeque;
use std::io::Write;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::Range;
use std::ptr;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::str::from_utf8_unchecked_mut;

const BUCKET_SIZE: usize = 1024 * 1024;

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct CRange {
    pub start: u32,
    pub end: u32,
}

pub fn newr(start: u32, end: u32) -> CRange {
    return CRange { start, end };
}

pub fn joinr(l: CRange, r: CRange) -> CRange {
    return newr(l.start, r.end);
}

impl CRange {
    pub fn into_range(self) -> Range<usize> {
        return (self.start as usize)..(self.end as usize);
    }
}

impl std::fmt::Debug for CRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("")
            .field(&self.start)
            .field(&self.end)
            .finish()
    }
}

impl std::fmt::Display for CRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("")
            .field(&self.start)
            .field(&self.end)
            .finish()
    }
}

#[derive(Debug)]
pub struct Error<'a> {
    pub location: CRange,
    pub message: &'a str,
}

pub fn err<'a, T>(loc: CRange, msg: &'a str) -> Result<T, Error<'a>> {
    return Err(Error {
        location: loc,
        message: msg,
    });
}

pub fn unwrap_err<'a, T>(
    value: Option<T>,
    location: CRange,
    message: &'a str,
) -> Result<T, Error<'a>> {
    if let Some(val) = value {
        return Ok(val);
    } else {
        return Err(Error { location, message });
    }
}

pub fn mut_ref_to_slice<T>(data: &mut T) -> &mut [T] {
    return unsafe { from_raw_parts_mut(data, 1) };
}

pub fn ref_to_slice<T>(data: &T) -> &[T] {
    return unsafe { from_raw_parts(data, 1) };
}

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
        // @Correctness panics in debug mode without this check
        let size = if size != 0 {
            (size - 1) / 16 * 16 + 16
        } else {
            size
        };
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
        unsafe {
            let location = self.new_unsafe(size_of::<T>()) as *mut T;
            ptr::write(location, t);
            return &mut *location;
        };
    }

    pub fn add_str(&mut self, values: &str) -> &'a mut str {
        let values = values.as_bytes();
        let len = values.len();
        let begin = unsafe { self.new_unsafe(values.len()) };
        let mut location = begin;
        for value in values {
            unsafe {
                *location = *value;
                location = location.add(1);
            }
        }

        return unsafe { from_utf8_unchecked_mut(from_raw_parts_mut(begin, len)) };
    }

    pub fn add_array<T>(&mut self, values: Vec<T>) -> &'a mut [T] {
        let size = size_of::<T>();
        let len = values.len();
        let begin = unsafe { self.new_unsafe(values.len() * size) as *mut T };
        let mut location = begin;
        for value in values {
            unsafe {
                ptr::write(location, value);
                location = location.add(1);
            }
        }

        return unsafe { from_raw_parts_mut(begin, len) };
    }
}

pub struct StringWriter {
    buf: Vec<u8>,
}

impl StringWriter {
    pub fn new() -> StringWriter {
        StringWriter {
            buf: Vec::with_capacity(8 * 1024),
        }
    }

    pub fn to_string(self) -> String {
        if let Ok(s) = String::from_utf8(self.buf) {
            s
        } else {
            String::new()
        }
    }
}

impl Write for StringWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for b in buf {
            self.buf.push(*b);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct Void {}

impl Void {
    pub fn new() -> Self {
        return Self {};
    }
}

impl Write for Void {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// Credit to unescape crate
macro_rules! try_option {
    ($o:expr) => {
        match $o {
            Some(s) => s,
            None => return None,
        }
    };
}

pub fn unescape(s: &str) -> Option<String> {
    let mut queue: VecDeque<_> = String::from(s).chars().collect();
    let mut s = String::new();

    while let Some(c) = queue.pop_front() {
        if c != '\\' {
            s.push(c);
            continue;
        }

        match queue.pop_front() {
            Some('b') => s.push('\u{0008}'),
            Some('f') => s.push('\u{000C}'),
            Some('n') => s.push('\n'),
            Some('r') => s.push('\r'),
            Some('t') => s.push('\t'),
            Some('\'') => s.push('\''),
            Some('\"') => s.push('\"'),
            Some('\\') => s.push('\\'),
            Some('u') => s.push(try_option!(unescape_unicode(&mut queue))),
            Some('x') => s.push(try_option!(unescape_byte(&mut queue))),
            Some(c) if c.is_digit(8) => s.push(try_option!(unescape_octal(c, &mut queue))),
            Some(c) => s.push(c),
            None => panic!(), // this case should never happen
        };
    }

    Some(s)
}

fn unescape_unicode(queue: &mut VecDeque<char>) -> Option<char> {
    let mut s = String::new();

    for _ in 0..4 {
        s.push(try_option!(queue.pop_front()));
    }

    let u = try_option!(u32::from_str_radix(&s, 16).ok());
    char::from_u32(u)
}

fn unescape_byte(queue: &mut VecDeque<char>) -> Option<char> {
    let mut s = String::new();

    for _ in 0..2 {
        s.push(try_option!(queue.pop_front()));
    }

    let u = try_option!(u32::from_str_radix(&s, 16).ok());
    char::from_u32(u)
}

fn unescape_octal(c: char, queue: &mut VecDeque<char>) -> Option<char> {
    match unescape_octal_leading(c, queue) {
        Some(ch) => {
            let _ = queue.pop_front();
            let _ = queue.pop_front();
            Some(ch)
        }
        None => unescape_octal_no_leading(c, queue),
    }
}

fn unescape_octal_leading(c: char, queue: &VecDeque<char>) -> Option<char> {
    if c != '0' && c != '1' && c != '2' && c != '3' {
        return None;
    }

    let mut s = String::new();
    s.push(c);
    s.push(*try_option!(queue.get(0)));
    s.push(*try_option!(queue.get(1)));

    let u = try_option!(u32::from_str_radix(&s, 8).ok());
    char::from_u32(u)
}

fn unescape_octal_no_leading(c: char, queue: &mut VecDeque<char>) -> Option<char> {
    let mut s = String::new();
    s.push(c);
    s.push(try_option!(queue.pop_front()));

    let u = try_option!(u32::from_str_radix(&s, 8).ok());
    char::from_u32(u)
}
