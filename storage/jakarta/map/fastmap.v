
const (
    // Number of bits from the hash stored for each entry
	hashbits            = 24
	// Number of bits from the hash stored for rehashing
	max_cached_hashbits = 16
	// Initial log-number of buckets in the hashtable
	init_log_capicity   = 5
	// Initial number of buckets in the hashtable
	init_capicity       = 1 << init_log_capicity
	// Maximum load-factor (len / capacity)
	max_load_factor     = 0.8
	// Initial highest even index in metas
	init_even_index     = init_capicity - 2
	// Used for incrementing `extra_metas` when max
	// probe count is too high, to avoid overflow
	extra_metas_inc     = 4
	// Bitmask to select all the hashbits
	hash_mask           = u32(0x00FFFFFF)
	// Used for incrementing the probe-count
	probe_inc           = u32(0x01000000)

    /**************************************/
    min_init_capacity   = 128
)
struct FastMapValue {
    
}

struct FastMapCursor{

}

struct FastMapRecord{

}

pub struct FastMap {
    // Number of Load Factor
    load_factor f64

	// Number of bytes of a key
	key_bytes int
	// Number of bytes of a value
	value_bytes int
mut:
    // Key Type in HashMap
    key Key
    //
    value FastMapValue
    //
    value2 FastMapValue
    //
    value3 FastMapValue
    //
    cursor FastMapCursor
    //
    record FastMapRecord
    // 
    valueColumnCount int
    //
    hashFunction HashFunction
    //
    keyBlockOffset int
    //
    keyDataOffset int
    //
    maxResizes int
    //
    initialKeyCapacity int
    //
    initialPageSize int
    // 
    capacity i128
    //
    offsets DirectLongList
    // 
    k_start i128
    //
    k_limit i128
    //
    k_pos i128
    //
    free int
    // 
    key_capacity int
    // 
    size int = 0;
    //
    mask int
    //
    n_resizes int

	// Highest even index in the fastmap
	// Extra metas that allows for no ranging when incrementing
	// index in the hashmap
pub mut:
	// Number of key-values currently in the hashmap
	len int
}

fn new_map(page_size int, key_types ColumnTypes, value_types ColumnTypes, key_capacity int, 
            load_factor f64, hashFunction HashFunction, max_resizes int) FastMap {
    return FastMap {
        assert page_size > 3;
        assert load_factor > 0 && load_factor < 1.0;
        init_key_capacity := key_capacity;
        init_page_size := page_size;
        load_factor := load_factor;
        k_start := k_pos = unsafe{ malloc( capacity = page_size, MemoryTag.NATIVE_FAST_MAP) };
        k_limit := k_start + page_size;
        key_capacity := key_capacity / load_factor;
        key_capacity := key_capacity < MIN_INITIAL_CAPACITY ? MIN_INITIAL_CAPACITY : Numbers.ceilPow2(this.keyCapacity);
        mask := key_capacity - 1;
        free := key_capacity * load_factor;
        offsets := new DirectLongList(this.keyCapacity, MemoryTag.NATIVE_FAST_MAP_LONG_LIST);
        offsets.setPos(key_capacity);
        offsets.zero(-1);
        hashFunction := hashFunction;
        n_resizes := 0;
        max_resizes := max_resizes;

        value_offsets := []int{};
        offset int = 4;
        if (valueTypes != null) {
            value_column_count = value_types.getColumnCount();
            columnSplit int = valueColumnCount;
            value_offsets = new int[columnSplit];

            for (int i = 0; i < columnSplit; i++) {
                value_offsets[i] = offset;
                columnType int = valueTypes.getColumnType(i);
                switch (ColumnType.tagOf(columnType)) {
                    case ColumnType.BYTE:
                    case ColumnType.BOOLEAN:
                    case ColumnType.GEOBYTE:
                        offset++
                        
                    case ColumnType.SHORT:
                    case ColumnType.CHAR:
                    case ColumnType.GEOSHORT:
                        offset += 2
    
                    case ColumnType.INT:
                    case ColumnType.FLOAT:
                    case ColumnType.SYMBOL:
                    case ColumnType.GEOINT:
                        offset += 4

                    case ColumnType.LONG:
                    case ColumnType.DOUBLE:
                    case ColumnType.DATE:
                    case ColumnType.TIMESTAMP:
                    case ColumnType.GEOLONG:
                        offset += 8

                    case ColumnType.LONG256:
                        offset += Long256.BYTES;
                        
                    default:
                        close();
                        throw CairoException.instance(0).put("value type is not supported: ").put(ColumnType.nameOf(columnType));
                }
            }
            value := FastMapValue(value_offsets);
            value2 := FastMapValue(value_offsets);
            value3 := FastMapValue(value_offsets);
            key_block_offset := offset;
            key_data_offset := key_block_offset + 4 * key_types.getColumnCount();
            record = FastMapRecord(value_offsets, column_split, key_data_offset, key_block_offset, value, key_types);
        } else {
            value_column_count := 0;
            value := FastMapValue();
            value2 := FastMapValue();
            value3 := FastMapValue();
            key_block_offset := offset;
            key_data_offset = key_block_offset + 4 * key_types.getColumnCount();
            record = FastMapRecord(null, 0, key_data_offset, key_block_offset, value, key_types);
        }
        assert key_block_offset < k_limit - k_start : "page size is too small for number of columns";
        cursor = FastMapCursor(record, this);
    }
}

pub impl FastMap {

     static  HashFunction DEFAULT_HASH = Hash::hashMem;
    //  static  int MIN_INITIAL_CAPACITY = 128;
      Key key = new Key();
     

      

    public FastMap(
            int pageSize,
            @Transient @NotNull ColumnTypes keyTypes,
            int keyCapacity,
            double loadFactor,
            int maxResizes
    ) {
        this(pageSize, keyTypes, null, keyCapacity, loadFactor, DEFAULT_HASH, maxResizes);
    }

    public FastMap(
            int pageSize,
            @Transient @NotNull ColumnTypes keyTypes,
            @Transient @Nullable ColumnTypes valueTypes,
            int keyCapacity,
            double loadFactor,
            int maxResizes
    ) {
        this(pageSize, keyTypes, valueTypes, keyCapacity, loadFactor, DEFAULT_HASH, maxResizes);
    }

    FastMap(
            int pageSize,
            @Transient ColumnTypes keyTypes,
            @Transient ColumnTypes valueTypes,
            int keyCapacity,
            double loadFactor,
            HashFunction hashFunction,
            int maxResizes
    ) {
        assert pageSize > 3;
        assert loadFactor > 0 && loadFactor < 1d;
        this.initialKeyCapacity = keyCapacity;
        this.initialPageSize = pageSize;
        this.loadFactor = loadFactor;
        this.kStart = kPos = Unsafe.malloc(this.capacity = pageSize, MemoryTag.NATIVE_FAST_MAP);
        this.kLimit = kStart + pageSize;
        this.keyCapacity = (int) (keyCapacity / loadFactor);
        this.keyCapacity = this.keyCapacity < MIN_INITIAL_CAPACITY ? MIN_INITIAL_CAPACITY : Numbers.ceilPow2(this.keyCapacity);
        this.mask = this.keyCapacity - 1;
        this.free = (int) (this.keyCapacity * loadFactor);
        this.offsets = new DirectLongList(this.keyCapacity, MemoryTag.NATIVE_FAST_MAP_LONG_LIST);
        this.offsets.setPos(this.keyCapacity);
        this.offsets.zero(-1);
        this.hashFunction = hashFunction;
        this.nResizes = 0;
        this.maxResizes = maxResizes;

        int[] valueOffsets;
        int offset = 4;
        if (valueTypes != null) {
            this.valueColumnCount = valueTypes.getColumnCount();
             int columnSplit = valueColumnCount;
            valueOffsets = new int[columnSplit];

            for (int i = 0; i < columnSplit; i++) {
                valueOffsets[i] = offset;
                 int columnType = valueTypes.getColumnType(i);
                switch (ColumnType.tagOf(columnType)) {
                    case ColumnType.BYTE:
                    case ColumnType.BOOLEAN:
                    case ColumnType.GEOBYTE:
                        offset++;
                        break;
                    case ColumnType.SHORT:
                    case ColumnType.CHAR:
                    case ColumnType.GEOSHORT:
                        offset += 2;
                        break;
                    case ColumnType.INT:
                    case ColumnType.FLOAT:
                    case ColumnType.SYMBOL:
                    case ColumnType.GEOINT:
                        offset += 4;
                        break;
                    case ColumnType.LONG:
                    case ColumnType.DOUBLE:
                    case ColumnType.DATE:
                    case ColumnType.TIMESTAMP:
                    case ColumnType.GEOLONG:
                        offset += 8;
                        break;
                    case ColumnType.LONG256:
                        offset += Long256.BYTES;
                        break;
                    default:
                        close();
                        throw CairoException.instance(0).put("value type is not supported: ").put(ColumnType.nameOf(columnType));
                }
            }
            this.value = new FastMapValue(valueOffsets);
            this.value2 = new FastMapValue(valueOffsets);
            this.value3 = new FastMapValue(valueOffsets);
            this.keyBlockOffset = offset;
            this.keyDataOffset = this.keyBlockOffset + 4 * keyTypes.getColumnCount();
            this.record = new FastMapRecord(valueOffsets, columnSplit, keyDataOffset, keyBlockOffset, value, keyTypes);
        } else {
            this.valueColumnCount = 0;
            this.value = new FastMapValue(null);
            this.value2 = new FastMapValue(null);
            this.value3 = new FastMapValue(null);
            this.keyBlockOffset = offset;
            this.keyDataOffset = this.keyBlockOffset + 4 * keyTypes.getColumnCount();
            this.record = new FastMapRecord(null, 0, keyDataOffset, keyBlockOffset, value, keyTypes);
        }
        assert this.keyBlockOffset < kLimit - kStart : "page size is too small for number of columns";
        this.cursor = new FastMapCursor(record, this);
    }

    
    pub fn clear() {
        k_pos = k_start;
        free = key_capacity * load_factor;
        size = 0;
        offsets.zero(-1);
    }

    // 内存close 等于内存free释放
    pub fn close() {
        offsets = unsafe { free(offsets) };
        if (k_start != 0) {
            unsafe{ free( k_start, capacity, MemoryTag.NATIVE_FAST_MAP) };
            k_start = 0;
        }
    }

    pub getCursor() RecordCursor {
        return cursor.init(kStart, size);
    }

    pub getRecord() MapRecord {
        return record;
    }

    pub size() i128 {
        return size;
    }

    pub valueAt(address i128) MapValue {
        return valueOf(address, false, this.value);
    }

    pub withKey() MapKey {
        return key.init();
    }

    pub fn restoreInitialCapacity() {
        k_start := k_pos = unsafe { realloc(k_start, k_limit - k_start, capacity = initial_page_size, MemoryTag.NATIVE_FAST_MAP) };
        k_limit := k_start + initial_page_size;
        key_capacity = initial_key_capacity / load_factor;
        key_capacity = key_capacity < MIN_INITIAL_CAPACITY ? MIN_INITIAL_CAPACITY : Numbers.ceilPow2(key_capacity);
        mask = key_capacity - 1;
        free = key_capacity * load_factor;
        offsets.extend(key_capacity);
        offsets.setPos(key_capacity);
        offsets.zero(-1);
        n_resizes = 0;
    }

    pub fn getAreaSize() i128 {
        return kLimit - kStart;
    }

    pub int getKeyCapacity() {
        return keyCapacity;
    }

    fn eqMixed(a i128, b i128, lim i128) bool {
        while (b < lim - 8) {
            if (Unsafe.getUnsafe().getLong(a) != Unsafe.getUnsafe().getLong(b)) {
                return false;
            }
            a += 8;
            b += 8;
        }

        while (b < lim) {
            if (Unsafe.getUnsafe().getByte(a++) != Unsafe.getUnsafe().getByte(b++)) {
                return false;
            }
        }
        return true;
    }

    fn eqLong(a i128, b i128, lim i128) bool {
        while (b < lim) {
            if (Unsafe.getUnsafe().getLong(a) != Unsafe.getUnsafe().getLong(b)) {
                return false;
            }
            a += 8;
            b += 8;
        }
        return true;
    }

    fn eqInt(a i128, b i128, lim i128) bool {
        while (b < lim) {
            if (Unsafe.getUnsafe().getInt(a) != Unsafe.getUnsafe().getInt(b)) {
                return false;
            }
            a += 4;
            b += 4;
        }
        return true;
    }

     FastMapValue asNew(keyWriter Key, index int, value FastMapValue) {
        kPos = keyWriter.appendAddress;
        offsets.set(index, keyWriter.startAddress - kStart);
        if (--free == 0) {
            rehash();
        }
        size++;
        return valueOf(keyWriter.startAddress, true, value);
    }

    fn eq(keyWriter Key, offset i128) bool {
        a i128 = kStart + offset;
        b i128 = keyWriter.startAddress;

        // check length first
        if (int(a) != int(b)) {
            return false;
        }

        lim i128 = b + key_Writer.len;

        // skip to the data
        a += keyDataOffset;
        b += keyDataOffset;

        d i128 = lim - b;
        if (d % Long.BYTES == 0) {
            return eqLong(a, b, lim);
        }

        if (d % Integer.BYTES == 0) {
            return eqInt(a, b, lim);
        }

        return eqMixed(a, b, lim);
    }

    fn getAppendOffset() i128 {
        return kPos;
    }

    fn getValueColumnCount() int {
        return valueColumnCount;
    }

    fn keyIndex() int {
        return hashFunction.hash(key.startAddress + keyDataOffset, key.len - keyDataOffset) & mask;
    }

    fn probe0( keyWriter Key, index int, value FastMapValue) FastMapValue {
        offset i128
        while ((offset = offsets.get(index = (++index & mask))) != -1) {
            if (eq(keyWriter, offset)) {
                return valueOf(kStart + offset, false, value);
            }
        }
        return asNew(keyWriter, index, value);
    }

    fn probeReadOnly(keyWriter Key, index int, value FastMapValue) FastMapValue {
        offset i128
        while ((offset = offsets.get(index = (++index & mask))) != -1) {
            if (eq(keyWriter, offset)) {
                return valueOf(kStart + offset, false, value);
            }
        }
        return null;
    }

    fn rehash() {
        capacity int := keyCapacity << 1;
        mask = capacity - 1;
        DirectLongList pointers = new DirectLongList(capacity, MemoryTag.NATIVE_FAST_MAP_LONG_LIST);
        pointers.setPos(capacity);
        pointers.zero(-1);

        for (long i = 0, k = this.offsets.size(); i < k; i++) {
            mut offset i128 = this.offsets.get(i);
            if (offset == -1) {
                continue;
            }
            mut index int = hashFunction.hash(kStart + offset + keyDataOffset, Unsafe.getUnsafe().getInt(kStart + offset) - keyDataOffset) & mask;
            while (pointers.get(index) != -1) {
                index = (index + 1) & mask;
            }
            pointers.set(index, offset);
        }
        offsets.close();
        offsets = pointers;
        free += (capacity - keyCapacity) * load_factor;
        key_capacity = capacity;
    }

    fn resize(size int) {
        if (nResizes < maxResizes) {
            nResizes++;
            kCapacity i128 := (kLimit - kStart) << 1;
            target i128 = key.appendAddress + size - kStart;
            if (kCapacity < target) {
                kCapacity = Numbers.ceilPow2(target);
            }
            k_address i128 = unsafe{ realloc(this.kStart, this.capacity, kCapacity, MemoryTag.NATIVE_FAST_MAP) };

            capacity = k_capacity;
            d i128 := kAddress - this.kStart;
            kPos += d;
            colOffsetDelta i128 = key.nextColOffset - key.startAddress;
            key.startAddress += d;
            key.appendAddress += d;
            key.nextColOffset = key.startAddress + colOffsetDelta;

            assert kPos > 0;
            assert key.startAddress > 0;
            assert key.appendAddress > 0;
            assert key.nextColOffset > 0;

            kStart = kAddress;
            kLimit = kAddress + kCapacity;
        } else {
            throw LimitOverflowException.instance().put("limit of ").put(maxResizes).put(" resizes exceeded in FastMap");
        }
    }

    fn value_of(address i128, _new bool, value FastMapValue) FastMapValue {
        return value.of(address, _new);
    }

    pub interface HashFunction {
        hash(address i128, len int) int;
    }

    pub struct Key implements MapKey {
         startAddress i128;
         appendAddress i128;
         len int;
        nextColOffset i128;

        pub fn createValue() MapValue {
            return createValue(value);
        }

        pub fn createValue2() MapValue {
            return createValue(value2);
        }

        pub fn createValue3() MapValue {
            return createValue(value3);
        }

        pub fn findValue() MapValue {
            return findValue(value);
        }

        pub fn findValue2() MapValue {
            return findValue(value2);
        }

        pub fn findValue3() MapValue {
            return findValue(value3);
        }

        pub fn put(record Record, sink RecordSink) {
            sink.copy(record, this);
        }

        pub fn init() Key {
            startAddress = kPos;
            appendAddress = kPos + keyDataOffset;
            nextColOffset = kPos + keyBlockOffset;
            return this;
        }

        pub fn putBin(value BinarySequence) {
            if (value == null) {
                putNull();
            } else {
                len i128 := value.length() + 4;
                if (len > Integer.MAX_VALUE) {
                    throw CairoException.instance(0).put("binary column is too large");
                }

                checkSize((int) len);
                l int = (int) (len - 4);
                Unsafe.getUnsafe().putInt(appendAddress, l);
                value.copyTo(appendAddress + 4L, 0L, l);
                appendAddress += len;
                writeOffset();
            }
        }

        pub fn putBool(value bool) {
            checkSize(1);
            Unsafe.getUnsafe().putByte(appendAddress, (byte) (value ? 1 : 0));
            appendAddress += 1;
            writeOffset();
        }

        pub fn putByte(value byte) {
            checkSize(1);
            Unsafe.getUnsafe().putByte(appendAddress, value);
            appendAddress += 1;
            writeOffset();
        }

        pub fn putDate(value i128) {
            putLong(value);
        }

        pub fn putDouble(value f64) {
            checkSize(Double.BYTES);
            Unsafe.getUnsafe().putDouble(appendAddress, value);
            appendAddress += Double.BYTES;
            writeOffset();
        }

        pub fn putFloat(value f32) {
            checkSize(Float.BYTES);
            Unsafe.getUnsafe().putFloat(appendAddress, value);
            appendAddress += Float.BYTES;
            writeOffset();
        }

        pub fn putInt(value int) {
            checkSize(Integer.BYTES);
            Unsafe.getUnsafe().putInt(appendAddress, value);
            appendAddress += Integer.BYTES;
            writeOffset();
        }

        pub fn putLong(value i128) {
            checkSize(Long.BYTES);
            Unsafe.getUnsafe().putLong(appendAddress, value);
            appendAddress += Long.BYTES;
            writeOffset();
        }

        pub fn putLong256(value u128) {
            checkSize(Long256.BYTES);
            Unsafe.getUnsafe().putLong(appendAddress, value.getLong0());
            Unsafe.getUnsafe().putLong(appendAddress + Long.BYTES, value.getLong1());
            Unsafe.getUnsafe().putLong(appendAddress + Long.BYTES * 2, value.getLong2());
            Unsafe.getUnsafe().putLong(appendAddress + Long.BYTES * 3, value.getLong3());
            appendAddress += Long256.BYTES;
            writeOffset();
        }

        
        pub fn putShort(short value ) {
            checkSize(2);
            Unsafe.getUnsafe().putShort(appendAddress, value);
            appendAddress += 2;
            writeOffset();
        }

        pub fn putChar(value string) {
            checkSize(Character.BYTES);
            Unsafe.getUnsafe().putChar(appendAddress, value);
            appendAddress += Character.BYTES;
            writeOffset();
        }

        pub fn putStr(value CharSequence) {
            if (value == null) {
                putNull();
                return;
            }

            len int := value.length();
            checkSize((len << 1) + 4);
            Unsafe.getUnsafe().putInt(appendAddress, len);
            appendAddress += 4;
            for (int i = 0; i < len; i++) {
                Unsafe.getUnsafe().putChar(appendAddress + ((long) i << 1), value.charAt(i));
            }
            appendAddress += (long) len << 1;
            writeOffset();
        }

        pub fn putStr(value CharSequence, lo int, hi int) {
            len int := hi - lo;
            checkSize((len << 1) + 4);
            Unsafe.getUnsafe().putInt(appendAddress, len);
            appendAddress += 4;
            for (int i = lo; i < hi; i++) {
                Unsafe.getUnsafe().putChar(appendAddress + ((long) (i - lo) << 1), value.charAt(i));
            }
            appendAddress += (long) len << 1;
            writeOffset();
        }

        pub fn putStrLowerCase(value CharSequence) {
            if (value == null) {
                putNull();
                return;
            }

            len int := value.length();
            checkSize((len << 1) + 4);
            Unsafe.getUnsafe().putInt(appendAddress, len);
            appendAddress += 4;
            for (int i = 0; i < len; i++) {
                Unsafe.getUnsafe().putChar(appendAddress + ((long) i << 1), Character.toLowerCase(value.charAt(i)));
            }
            appendAddress += (long) len << 1;
            writeOffset();
        }

        pub fn putStrLowerCase(value CharSequence, lo int, hi int) {
            len int := hi - lo;
            checkSize((len << 1) + 4);
            Unsafe.getUnsafe().putInt(appendAddress, len);
            appendAddress += 4;
            for (int i = lo; i < hi; i++) {
                Unsafe.getUnsafe().putChar(appendAddress + ((long) (i - lo) << 1), Character.toLowerCase(value.charAt(i)));
            }
            appendAddress += (long) len << 1;
            writeOffset();
        }

        pub fn putRecord(value Record) {
            // noop
        }

        pub fn putTimestamp(value i128) {
            putLong(value);
        }

        pub fn skip(bytes int) {
            checkSize(bytes);
            appendAddress += bytes;
            writeOffset();
        }

        fn checkSize(size int) {
            if (appendAddress + size > kLimit) {
                resize(size);
            }
        }

        fn commit() {
            Unsafe.getUnsafe().putInt(startAddress, len = (int) (appendAddress - startAddress));
        }

        fn createValue(value FastMapValue) MapValue {
            commit();
            // calculate hash remembering "key" structure
            // [ len | value block | key offset block | key data block ]
            index int := keyIndex();
            offset i128 := offsets.get(index);

            if (offset == -1) {
                return asNew(this, index, value);
            } else if (eq(this, offset)) {
                return valueOf(kStart + offset, false, value);
            } else {
                return probe0(this, index, value);
            }
        }

        fn findValue(value FastMapValue) MapValue {
            commit();
            index int := keyIndex();
            offset i128 := offsets.get(index);

            if (offset == -1) {
                return null;
            } else if (eq(this, offset)) {
                return valueOf(kStart + offset, false, value);
            } else {
                return probeReadOnly(this, index, value);
            }
        }

        fn putNull() {
            checkSize(4);
            Unsafe.getUnsafe().putInt(appendAddress, TableUtils.NULL_LEN);
            appendAddress += 4;
            writeOffset();
        }

        fn writeOffset() {
            len i128 := appendAddress - startAddress;
            if (len > Integer.MAX_VALUE) {
                throw CairoException.instance(0).put("row data is too large");
            }
            Unsafe.getUnsafe().putInt(nextColOffset, (int) len);
            nextColOffset += 4;
        }
    }
}