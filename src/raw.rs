use crate::traits::RawDoubleBuffer;

unsafe impl<B> RawDoubleBuffer for [B; 2] {
    type Buffer = B;

    unsafe fn split(this: *mut Self, which: bool) -> (*mut Self::Buffer, *const Self::Buffer) {
        let ptr = this as *mut B;
        let writer = usize::from(which);
        let reader = usize::from(!which);
        (ptr.add(writer), ptr.add(reader))
    }
}

unsafe impl<B> RawDoubleBuffer for [B] {
    type Buffer = [B];

    unsafe fn split(this: *mut Self, which: bool) -> (*mut Self::Buffer, *const Self::Buffer) {
        let len = (*this).len() / 2;
        let ptr = this as *mut B;

        let writer = usize::from(which);
        let reader = usize::from(!which);

        let writer = ptr.add(len * writer);
        let reader = ptr.add(len * reader);

        (
            core::ptr::slice_from_raw_parts_mut(writer, len),
            core::ptr::slice_from_raw_parts_mut(reader, len),
        )
    }
}
