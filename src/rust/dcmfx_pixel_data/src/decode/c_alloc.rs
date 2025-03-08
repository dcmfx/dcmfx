use alloc::alloc::Layout;

/// Provides a C-compatible `malloc()` function that maps through to the global
/// allocator.
/// 
/// This is used on WASM platforms so that image codec libraries written in C
/// can allocate memory.
///
#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: usize) -> *mut u8 {
  let layout_size = ::core::mem::size_of::<Layout>();

  let layout = Layout::from_size_align(
    layout_size + size,
    ::core::mem::align_of::<usize>(),
  )
  .unwrap();

  let ptr = unsafe { alloc::alloc::alloc(layout) };
  if ptr.is_null() {
    return ptr;
  }

  // Copy layout into the initial bytes
  unsafe {
    *(ptr as *mut Layout) = layout;
  }

  // Return pointer to the data following the layout definition
  unsafe { ptr.add(layout_size) }
}

/// Provides a C-compatible `free()` function that maps through to the global
/// allocator.
///
#[unsafe(no_mangle)]
pub extern "C" fn free(ptr: *mut u8) {
  if ptr.is_null() {
    return;
  }

  let layout_size = ::core::mem::size_of::<Layout>();

  unsafe {
    let real_ptr = ptr.sub(layout_size);

    // Read layout from the start of the allocation
    let layout: Layout = *(real_ptr as *const Layout);

    alloc::alloc::dealloc(real_ptr, layout)
  }
}
