//! This module provides C-compatible allocation functions that map through to
//! Rust's global allocator.
//!
//! These are needed on no_std targets, e.g. WASM, so that the image codec
//! libraries such as OpenJPEG can allocate memory.

use alloc::alloc::Layout;

#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: usize) -> *mut u8 {
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
      return ptr;
    }

    // Copy layout into the initial bytes
    *(ptr as *mut Layout) = layout;

    // Return pointer to the data following the layout definition
    ptr.add(layout_size)
  }
}

#[unsafe(no_mangle)]
pub extern "C" fn calloc(count: usize, size: usize) -> *mut u8 {
  let ptr = malloc(count * size);

  // Zero the allocated bytes
  if !ptr.is_null() {
    unsafe {
      ::core::ptr::write_bytes(ptr, 0, count * size);
    }
  }

  ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn realloc(ptr: *mut u8, new_size: usize) -> *mut u8 {
  if ptr.is_null() {
    return malloc(new_size);
  }

  let layout_size = ::core::mem::size_of::<Layout>();

  unsafe {
    let ptr = ptr.sub(layout_size);

    // Read current layout from the start of the allocation
    let layout: Layout = *(ptr as *const Layout);

    // Perform reallocation
    let ptr = alloc::alloc::realloc(ptr, layout, layout_size + new_size);
    if ptr.is_null() {
      return ptr;
    }

    // Set new layout into the initial bytes
    *(ptr as *mut Layout) =
      Layout::from_size_align(new_size, layout.align()).unwrap();

    // Return pointer to the data following the layout definition
    ptr.add(layout_size)
  }
}

#[unsafe(no_mangle)]
pub extern "C" fn posix_memalign(
  ptr: *mut *mut u8,
  _alignment: usize,
  size: usize,
) -> i32 {
  // TODO: respect aligned allocation requests

  let new_ptr = malloc(size);
  if new_ptr.is_null() {
    return 12; // ENOMEM
  }

  unsafe {
    *ptr = new_ptr;
  }

  0
}

#[unsafe(no_mangle)]
pub extern "C" fn free(ptr: *mut u8) {
  if ptr.is_null() {
    return;
  }

  let layout_size = ::core::mem::size_of::<Layout>();

  unsafe {
    let ptr = ptr.sub(layout_size);

    // Read layout from the start of the allocation
    let layout: Layout = *(ptr as *const Layout);

    alloc::alloc::dealloc(ptr, layout)
  }
}
