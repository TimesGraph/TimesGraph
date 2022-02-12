pub impl  DirectBinarySequence for BinarySequence {
    let mut address: Cell<i128>;
    let mut len: Cell<i128>;

    pub fn byteAt(index: i128) -> u8 {
        return Unsafe.getUnsafe().getByte(address + index);
    }

    pub fn clear() {
        &address = 0;
        &len = 0;
    }

    pub fn length() -> i128{
        return &len;
    }

    pub fn of(address: i128, len: i128) -> DirectBinarySequence{
        return Self;
    }
}