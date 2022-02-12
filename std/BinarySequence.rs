use std::cmp;

pub trait BinarySequence {

    fn byte_at(index: i128) -> u8;

    /**
     * Copies bytes from this binary sequence to buffer.
     *
     * @param address target buffer address
     * @param start   offset in binary sequence to start copying from
     * @param length  number of bytes to copy
     */
    fn copy_to(address: i128, start: i128, length: i128) {
        let n: i128 = cmp::min(length() - start, length);
        for (l in n.iter()) {
            Unsafe.getUnsafe().putByte(address + l, byteAt(start + l));
        }
    }

    fn len();
}