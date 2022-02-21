pub struct FastMapRecord {
    split int
    keyDataOffset int
    keyBlockOffset int
    valueOffsets []int
    csA []DirectCharSequence
    csB []DirectCharSequence
    bs []DirectBinarySequence
    long256A []Long256Impl
    long256B []Long256Impl
    value FastMapValue
    address0 i128
    address1 i128
    address2 i128
    symbolTableResolver RecordCursor
    symbolTableIndex IntList

    fn new (valueOffsets []int, split int, keyDataOffset int, keyBlockOffset int,
                value FastMapValue, keyTypes ColumnTypes) FastMapRecord {
        valueOffsets := valueOffsets;
        split := split;
        keyBlockOffset := keyBlockOffset;
        keyDataOffset := keyDataOffset;
        value := value;
        value.linkRecord(this); // provides feature to position this record at location of map value

        n int := keyTypes.getColumnCount();

        DirectCharSequence[] csA
        DirectCharSequence[] csB
        DirectBinarySequence[] bs
        Long256Impl[] long256A
        Long256Impl[] long256B

        for (int i = 0; i < n; i++) {
            switch (ColumnType.tagOf(keyTypes.getColumnType(i))) {
                case ColumnType.STRING:
                    if (csA == null) {
                        csA = new DirectCharSequence[n + split];
                        csB = new DirectCharSequence[n + split];
                    }
                    csA[i + split] = new DirectCharSequence();
                    csB[i + split] = new DirectCharSequence();
                    break;
                case ColumnType.BINARY:
                    if (bs == null) {
                        bs = new DirectBinarySequence[n + split];
                    }
                    bs[i + split] = new DirectBinarySequence();
                    break;
                case ColumnType.LONG256:
                    if (long256A == null) {
                        long256A = new Long256Impl[n + split];
                        long256B = new Long256Impl[n + split];
                    }
                    long256A[i + split] = new Long256Impl();
                    long256B[i + split] = new Long256Impl();
                    break;
                default:
                    break;
            }
        }

        this.csA = csA;
        this.csB = csB;
        this.bs = bs;
        this.long256A = long256A;
        this.long256B = long256B;
    }

    private FastMapRecord(
            int[] valueOffsets,
            int split,
            int keyDataOffset,
            int keyBlockOffset,
            DirectCharSequence[] csA,
            DirectCharSequence[] csB,
            DirectBinarySequence[] bs,
            Long256Impl[] long256A,
            Long256Impl[] long256B
    ) {

        value_offsets := value_offsets;
        split := split;
        key_block_offset := key_block_offset;
        key_data_offset := key_data_offset;
        value := new FastMapValue(valueOffsets);
        csA := csA;
        csB := csB;
        bs := bs;
        long256A := long256A;
        long256B := long256B;
    }

 
}

pub fn getBin(columnIndex int) BinarySequence {
    address i128 = addressOfColumn(columnIndex);
    len int = unsafe {to_int(address)};
    if (len == TableUtils.NULL_LEN) {
        return null;
    }
    bs DirectBinarySequence = bs[columnIndex];
    bs.of(address + 4, len);
    return bs;
}

pub fn getBinLen(columnIndex int) i128 {
    return unsafe{ to_int(addressOfColumn(columnIndex)) };
}

pub fn getBool(columnIndex int) bool {
    return unsafe{ to_bool(addressOfColumn(columnIndex)) };
}

pub fn getByte(columnIndex int) byte {
    return unsafe{ to_byte(addressOfColumn(columnIndex)) };
}

pub fn getChar(columnIndex int) string {
    return unsafe{ to_char(addressOfColumn(columnIndex)) };
}

pub fn getDouble(columnIndex int) f64 {
    return unsafe{ to_f64(addressOfColumn(columnIndex)) };
}

pub fn getFloat(columnIndex int) f32 {
    return unsafe{ to_f32(addressOfColumn(columnIndex)) };
}

pub fn getInt(columnIndex int) int {
    return unsafe{to_int(addressOfColumn(columnIndex))};
}

pub fn getLong(columnIndex int) i128 {
    return unsafe{ to_i128(addressOfColumn(columnIndex)) };
}

pub fn getLong256(columnIndex int, sink CharSink) {
    long address = addressOfColumn(columnIndex);
    final long a = Unsafe.getUnsafe().getLong(address);
    final long b = Unsafe.getUnsafe().getLong(address + Long.BYTES);
    final long c = Unsafe.getUnsafe().getLong(address + Long.BYTES * 2);
    final long d = Unsafe.getUnsafe().getLong(address + Long.BYTES * 3);
    Numbers.appendLong256(a, b, c, d, sink);
}

pub fn getLong256A(columnIndex int) i256{
    return getLong256Generic(long256A, columnIndex);
}

pub fn getLong256B(columnIndex int) i256 {
    return getLong256Generic(long256B, columnIndex);
}

pub fn getRowId() i128 {
    return address0;
}

pub fn getShort(columnIndex int) i8{
    return Unsafe.getUnsafe().getShort(addressOfColumn(columnIndex));
}

pub fn getStr(columnIndex int) CharSequence {
    assert columnIndex < csA.length;
    return getStr0(columnIndex, csA[columnIndex]);
}

pub fn getStr(columnIndex int, sink CharSink) {
    address i128 := addressOfColumn(columnIndex);
    len int := unsafe{to_int(address)};
    address += 4;
    for (int i = 0; i < len; i++) {
        sink.put(unsafe{to_char(address))};
        address += 2;
    }
}

pub fn getStrB(columnIndex int) CharSequence {
    return getStr0(columnIndex, csB[columnIndex]);
}

pub fn getStrLen(columnIndex int) int {
    return unsafe{to_int(addressOfColumn(columnIndex))};
}

pub fn getSym(col int) CharSequence {
    return symbolTableResolver.getSymbolTable(symbolTableIndex.getQuick(col)).valueOf(getInt(col));
}

pub fn getSymB(col int) CharSequence {
    return symbolTableResolver.getSymbolTable(symbolTableIndex.getQuick(col)).valueBOf(getInt(col));
}

pub fn getGeoByte(col int) byte {
    return getByte(col);
}

pub fn getGeoShort(col int) i8 {
    return getShort(col);
}

pub fn getGeoInt(col int) int {
    return getInt(col);
}

pub fn getGeoLong(col int) i128 {
    return getLong(col);
}

pub fn getValue() MapValue {
    return value.of(address0, false);
}

pub fn setSymbolTableResolver(resolver RecordCursor, symbolTableIndex IntList) {
    symbolTableResolver := resolver;
    symbolTableIndex := symbolTableIndex;
}

fn addressOfColumn(index int) i128 {

    if (index < split) {
        return address0 + valueOffsets[index];
    }

    if (index == split) {
        return address1;
    }

    return unsafe{ to_int(address2 + (index - split - 1) * 4L) + address0 };
}

@SuppressWarnings("MethodDoesntCallSuperMethod")
@Override
protected MapRecord clone() {
    final DirectCharSequence[] csA;
    final DirectCharSequence[] csB;
    final DirectBinarySequence[] bs;
    final Long256Impl[] long256A;
    final Long256Impl[] long256B;

    // csA and csB are pegged, checking one for null should be enough
    if (this.csA != null) {
        int n = this.csA.length;
        csA = new DirectCharSequence[n];
        csB = new DirectCharSequence[n];

        for (int i = 0; i < n; i++) {
            if (this.csA[i] != null) {
                csA[i] = new DirectCharSequence();
                csB[i] = new DirectCharSequence();
            }
        }
    } else {
        csA = null;
        csB = null;
    }

    if (this.bs != null) {
        int n = this.bs.length;
        bs = new DirectBinarySequence[n];
        for (int i = 0; i < n; i++) {
            if (this.bs[i] != null) {
                bs[i] = new DirectBinarySequence();
            }
        }
    } else {
        bs = null;
    }

    if (this.long256A != null) {
        int n = this.long256A.length;
        long256A = new Long256Impl[n];
        long256B = new Long256Impl[n];

        for (int i = 0; i < n; i++) {
            if (this.long256A[i] != null) {
                long256A[i] = new Long256Impl();
                long256B[i] = new Long256Impl();
            }
        }
    } else {
        long256A = null;
        long256B = null;
    }
    return new FastMapRecord(valueOffsets, split, keyDataOffset, keyBlockOffset, csA, csB, bs, long256A, long256B);
}

fn getLong256Generic(Long256Impl[] array , int columnIndex int) i256 {
    address i128 := addressOfColumn(columnIndex);
    Long256Impl long256 := array[columnIndex];
    long256.setAll(
            unsafe{ getLong(address) },
            unsafe{ getLong(address + Long.BYTES) },
            unsafe{ getLong(address + Long.BYTES * 2) },
            unsafe{ getLong(address + Long.BYTES * 3) }
    );
    return long256;
}

fn  getStr0(index int, cs DirectCharSequence) CharSequence {
    address i128 := addressOfColumn(index);
    len int := unsafe{to_int(address)};
    return len == TableUtils.NULL_LEN ? null : cs.of(address + 4, address + 4 + len * 2L);
}

fn of(address i128) {
    address0 := address;
    address1 := address + keyDataOffset;
    address2 := address + keyBlockOffset;
}