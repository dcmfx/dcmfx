//! This module provides C-compatible allocation functions that map through to
//! Rust's global allocator.
//!
//! These are needed on no_std targets, e.g. WASM, so that the image codec
//! libraries such as OpenJPEG can allocate memory.

use alloc::alloc::Layout;
use core::ffi::c_void;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
  let layout_size = ::core::mem::size_of::<Layout>();

  // Construct layout that has enough space for the allocation preceded by the
  // Layout instance
  let layout = Layout::from_size_align(
    layout_size + size,
    ::core::mem::align_of::<usize>(),
  )
  .unwrap();

  unsafe {
    // Allocate
    let ptr = alloc::alloc::alloc(layout);
    if ptr.is_null() {
      return ptr.cast();
    }

    // Copy layout into the initial bytes
    *(ptr as *mut Layout) = layout;

    // Return pointer to the data following the layout definition
    ptr.add(layout_size).cast()
  }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calloc(count: usize, size: usize) -> *mut c_void {
  unsafe {
    let ptr = malloc(count * size);

    // Zero the allocated bytes
    if !ptr.is_null() {
      ::core::ptr::write_bytes(ptr.cast::<u8>(), 0, count * size);
    }

    ptr
  }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn realloc(
  ptr: *mut c_void,
  new_size: usize,
) -> *mut c_void {
  if ptr.is_null() {
    return unsafe { malloc(new_size) };
  }

  let layout_size = ::core::mem::size_of::<Layout>();

  unsafe {
    let ptr = ptr.cast::<u8>().sub(layout_size);

    // Read current layout from the start of the allocation
    let layout: Layout = *(ptr as *const Layout);

    // Perform reallocation
    let ptr = alloc::alloc::realloc(ptr, layout, layout_size + new_size);
    if ptr.is_null() {
      return ptr.cast();
    }

    // Set new layout into the initial bytes
    *(ptr as *mut Layout) =
      Layout::from_size_align(new_size, layout.align()).unwrap();

    // Return pointer to the data following the layout definition
    ptr.add(layout_size).cast()
  }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
  if ptr.is_null() {
    return;
  }

  let layout_size = ::core::mem::size_of::<Layout>();

  unsafe {
    let ptr = ptr.cast::<u8>().sub(layout_size);

    // Read layout from the start of the allocation
    let layout: Layout = *(ptr as *const Layout);

    alloc::alloc::dealloc(ptr, layout)
  }
}
