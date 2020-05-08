use crate::syntax_tree::Type;
use std::alloc::{alloc, dealloc, Layout};
use std::collections::HashMap;
use std::io::Write;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::Range;
use std::ptr;
use std::ptr::NonNull;
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

#[derive(Clone, Copy)]
pub struct Bucket {
    begin: *mut u8,
    end: *mut u8,
}

pub fn mut_ref_to_slice<T>(data: &mut T) -> &mut [T] {
    return unsafe { from_raw_parts_mut(data, 1) };
}

pub fn ref_to_slice<T>(data: &T) -> &[T] {
    return unsafe { from_raw_parts(data, 1) };
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SymbolInfo<'a> {
    Function {
        uid: u32,
        return_type: &'a Type<'a>,
        arguments: &'a [Type<'a>],
        view: CRange,
    },
    Variable {
        uid: u32,
        type_: &'a Type<'a>,
        view: CRange,
    },
}

impl<'a> SymbolInfo<'a> {
    pub fn get_type(&self) -> Type<'a> {
        return match self {
            SymbolInfo::Function {
                return_type,
                arguments,
                ..
            } => Type::Function {
                return_type,
                arguments,
            },
            SymbolInfo::Variable { type_, .. } => **type_,
        };
    }

    pub fn view(&self) -> CRange {
        use SymbolInfo::*;
        return match self {
            Function { view, .. } => *view,
            Variable { view, .. } => *view,
        };
    }

    pub fn uid(&self) -> u32 {
        return match self {
            SymbolInfo::Function { uid, .. } => *uid,
            SymbolInfo::Variable { uid, .. } => *uid,
        };
    }
}

pub struct OffsetTable {
    pub uids: HashMap<u32, i32>,
    parent: Option<NonNull<OffsetTable>>,
}

impl OffsetTable {
    pub fn new_global(uids: HashMap<u32, i32>) -> Self {
        return Self { uids, parent: None };
    }

    pub fn new(parent: &OffsetTable) -> Self {
        return Self {
            uids: HashMap::new(),
            parent: Some(NonNull::from(parent)),
        };
    }

    pub fn declare(&mut self, symbol: u32, offset: i32) {
        if self.uids.contains_key(&symbol) {
            panic!();
        }
        self.uids.insert(symbol, offset);
    }

    pub fn search(&self, symbol: u32) -> Option<i32> {
        return unsafe { self.search_unsafe(symbol) };
    }

    unsafe fn search_unsafe(&self, symbol: u32) -> Option<i32> {
        let mut current = NonNull::from(self);
        let mut uids = NonNull::from(&current.as_ref().uids);

        loop {
            if let Some(info) = uids.as_ref().get(&symbol) {
                return Some(*info);
            } else if let Some(parent) = current.as_ref().parent {
                current = parent;
                uids = NonNull::from(&current.as_ref().uids);
            } else {
                return None;
            }
        }
    }
}

pub struct SymbolTable<'a> {
    pub symbols: HashMap<u32, SymbolInfo<'a>>,
    parent: Option<NonNull<SymbolTable<'a>>>,
}

impl<'a> SymbolTable<'a> {
    pub fn new_global(symbols: HashMap<u32, SymbolInfo<'a>>) -> Self {
        return Self {
            symbols,
            parent: None,
        };
    }

    pub fn new<'b>(parent: &SymbolTable<'b>) -> Self
    where
        'b: 'a,
    {
        return Self {
            symbols: HashMap::new(),
            parent: Some(NonNull::from(parent)),
        };
    }

    pub fn search_current(&self, symbol: u32) -> Option<SymbolInfo<'a>> {
        return self.symbols.get(&symbol).map(|r| *r);
    }

    pub fn fold_into_parent(mut self) -> Result<(), Error<'static>> {
        for (symbol, info) in self.symbols.drain() {
            unsafe { self.parent.unwrap().as_mut() }.declare(symbol, info)?;
        }
        return Ok(());
    }

    pub fn merge_parallel_tables<'b>(
        mut left: SymbolTable<'b>,
        mut right: SymbolTable<'b>,
    ) -> Result<SymbolTable<'b>, Error<'static>> {
        assert!(left.parent == right.parent && left.parent != None);
        let mut result = SymbolTable::new(unsafe { left.parent.unwrap().as_mut() });
        for (id, info) in left.symbols.drain() {
            if let Some(rinfo) = right.symbols.remove(&id) {
                if info != rinfo {
                    return err(info.view(), "");
                }
            }
            result.declare(id, info)?;
        }

        for (id, info) in right.symbols.drain() {
            result.declare(id, info)?;
        }

        return Ok(result);
    }

    pub fn declare(&mut self, symbol: u32, info: SymbolInfo<'a>) -> Result<(), Error<'static>> {
        if self.symbols.contains_key(&symbol) {
            return err(info.view(), "name already exists in scope");
        }
        self.symbols.insert(symbol, info);
        return Ok(());
    }

    pub fn search(&self, symbol: u32) -> Option<SymbolInfo<'a>> {
        return unsafe { self.search_unsafe(symbol) };
    }

    unsafe fn search_unsafe(&self, symbol: u32) -> Option<SymbolInfo<'a>> {
        let mut current = NonNull::from(self);
        let mut symbols = NonNull::from(&current.as_ref().symbols);

        loop {
            if let Some(info) = symbols.as_ref().get(&symbol) {
                return Some(*info);
            } else if let Some(parent) = current.as_ref().parent {
                current = parent;
                symbols = NonNull::from(&current.as_ref().symbols);
            } else {
                return None;
            }
        }
    }
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
