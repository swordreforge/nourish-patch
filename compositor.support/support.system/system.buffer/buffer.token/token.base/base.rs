use std::marker::PhantomData;

/// A system's self-addressed mutation queue token. Buffers are the ONLY path
/// to storage mutation: lifecycle methods (input/update/receive/...) read
/// storage and `cx.write(&BUF, message)` their intent; the kernel delivers it
/// back to the SAME system's `System::buffer()`, the one context holding
/// `&mut Storage`. The macro keeps the token entirely crate-private — only
/// the owning system can write or interpret its buffer.
/// The CONSTANT REFERENCE is the identity — no string identifiers.
pub struct Buffer<M: 'static> {
    _marker: PhantomData<fn() -> M>,
}

impl<M> Buffer<M> {
    pub const fn new() -> Self {
        Self { _marker: PhantomData }
    }
}

/// Declare a system's buffer message stream (crate-private on purpose).
///
/// ```ignore
/// y5_buffer!(CAMERA_BUF: CameraCommand);
/// ```
#[macro_export]
macro_rules! y5_buffer {
    ($name:ident : $M:ty) => {
        pub(crate) static $name: $crate::base::Buffer<$M> = $crate::base::Buffer::new();
    };
}
