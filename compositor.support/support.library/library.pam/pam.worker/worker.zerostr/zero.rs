/// Overwrite every byte with zero using volatile writes so the
/// compiler cannot optimize them away as dead stores.
#[inline(never)]
pub(crate) fn zero_bytes(bytes: &mut [u8]) {
    for b in bytes.iter_mut() {
        unsafe { std::ptr::write_volatile(b as *mut u8, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}
