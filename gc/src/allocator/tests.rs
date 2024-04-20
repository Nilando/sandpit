use super::arena::Arena;
use super::constants::{BLOCK_CAPACITY, BLOCK_SIZE};
use super::size_class::SizeClass;
use super::Allocator;
use crate::allocate::{Allocate, GenerationalArena};
use std::alloc::Layout;
use std::ptr::write;
use std::mem::{size_of, align_of};

#[test]
fn hello_alloc() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let name = "Hello Alloc";
    let layout = Layout::for_value(&name);

    assert_eq!(arena.get_size(), 0);

    let ptr = allocator.alloc(layout).unwrap();

    unsafe {
        write(ptr.as_ptr().cast(), name);

        let expect = ptr.cast::<&str>().as_ref();
        assert_eq!(expect, &name);
    }

    assert_eq!(arena.get_size(), BLOCK_SIZE);
}

#[test]
fn alloc_large() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let data: [usize; BLOCK_SIZE] = [0; BLOCK_SIZE];
    let layout = Layout::for_value(&data);

    assert_eq!(arena.get_size(), 0);

    allocator.alloc(layout).unwrap();
}

#[test]
fn alloc_many_single_bytes() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let layout = Layout::new::<u8>();

    for _ in 0..100_000 {
        let ptr = allocator.alloc(layout).unwrap();

        unsafe {
            write(ptr.as_ptr(), 3_u8);
            let val = ptr.as_ref();
            assert_eq!(*val, 3);
        }
    }
}

#[test]
fn alloc_too_big() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let layout = Layout::from_size_align(std::u32::MAX as usize, 8).unwrap();
    let result = allocator.alloc(layout);
    assert!(result.is_err());
}

#[test]
fn alloc_two_large_arrays() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let layout = Layout::from_size_align(BLOCK_CAPACITY / 2, 8).unwrap();
    allocator.alloc(layout).unwrap();
    assert_eq!(arena.get_size(), BLOCK_SIZE);
    allocator.alloc(layout).unwrap();
    assert_eq!(arena.get_size(), BLOCK_SIZE * 2);
}

#[test]
fn refresh_arena() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let layout = Layout::from_size_align(BLOCK_CAPACITY / 2, 8).unwrap();
    for _ in 0..20 {
        allocator.alloc(layout).unwrap();
    }
    assert!(arena.get_size() > 10 * BLOCK_SIZE);
    arena.refresh();
    assert_eq!(arena.get_size(), 10 * BLOCK_SIZE);
}

#[test]
fn object_align() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    for i in 0..10 {
        let align: usize = 2_usize.pow(i);
        let layout = Layout::from_size_align(32, align).unwrap();
        let ptr = allocator.alloc(layout).unwrap();

        assert!(((ptr.as_ptr() as usize) % align) == 0)
    }
}

#[test]
fn clone_size_class() {
    // this is just for test coverage
    let foo = SizeClass::get_for_size(69);
    let clone = foo.clone();

    assert!(foo == clone);
}

#[test]
fn large_object_align() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let layout = Layout::from_size_align(BLOCK_CAPACITY * 2, 128).unwrap();
    let ptr = allocator.alloc(layout).unwrap();

    assert!(((ptr.as_ptr() as usize) % 128) == 0)
}

#[test]
fn arena_get_size() {
    use super::header::{Mark, Header};
    let arena = Arena::new();
    let alloc = Allocator::new(&arena);

    let small = Layout::from_size_align(1, align_of::<u8>()).unwrap();
    let medium = Layout::from_size_align(512, align_of::<u8>()).unwrap();
    let large = Layout::from_size_align(80_000, align_of::<u8>()).unwrap();

    let p1 = alloc.alloc(small).unwrap();
    let p2 = alloc.alloc(medium).unwrap();
    let p3 = alloc.alloc(large).unwrap();
    Allocator::set_mark(p1, Mark::Red);
    Allocator::set_mark(p2, Mark::Red);
    Allocator::set_mark(p3, Mark::Red);

    let small_header = Allocator::get_header(p1);
    let med_header = Allocator::get_header(p2);
    let large_header = Allocator::get_header(p3);

    unsafe {
        assert_eq!((*small_header).get_size_class(), SizeClass::Small);
        assert_eq!((*med_header).get_size_class(), SizeClass::Medium);
        assert_eq!((*large_header).get_size_class(), SizeClass::Large);
    }

    let align = std::cmp::max(align_of::<Header>(), large.align());
    let header_size = size_of::<Header>();
    let padding = (align - (header_size % align)) % align;
    let large_size = header_size + padding + large.size();

    assert_eq!(arena.get_size(), (BLOCK_SIZE + large_size));
}
