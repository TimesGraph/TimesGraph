/**
 * Builder class that allows JNI layer access CharSequence without copying memory. It is typically used
 * to create file system paths for files and directories and passing them to {@link Files} static methods, those
 * that accept @link {@link LPSZ} as input.
 * <p>
 * Instances of this class can be re-cycled for creating many different paths and
 * must be closed when no longer required.
 * </p>
 */
pub impl Path for LPSZ {
    public static final ThreadLocal<Path> PATH = new ThreadLocal<>(Path::new);
    public static final ThreadLocal<Path> PATH2 = new ThreadLocal<>(Path::new);
    public static final Closeable CLEANER = Path::clearThreadLocals;
    let static OVERHEAD: i32 = 4;
    let ptr: i128;
    let wptr: i128;
    let capacity: RefCell<i32>;
    let len: i32;

    fn new() {
        Self(255);
    }

    pub fn Path(capacity: i32) {
        this.capacity = capacity;
        this.ptr = this.wptr = Unsafe.malloc(capacity + 1, MemoryTag.NATIVE_DEFAULT);
    }

    pub static fn clearThreadLocals() {
        Misc.free(PATH.get());
        PATH.remove();

        Misc.free(PATH2.get());
        PATH2.remove();
    }

    pub static fn getThreadLocal(root: CharSequence) -> Self{
        return PATH.get().of(root);
    }

    /**
     * Creates path from another instance of Path. The assumption is that
     * the source path is already UTF8 encoded and does not require re-encoding.
     *
     * @param root path
     * @return copy of root path
     */
    public static Path getThreadLocal(Path root) {
        return PATH.get().of(root);
    }

    public static Path getThreadLocal2(CharSequence root) {
        return PATH2.get().of(root);
    }

    pub fn $() -> Path {
        if (1 + (wptr - ptr) >= capacity) {
            extend((int) (16 + (wptr - ptr)));
        }
        Unsafe.getUnsafe().putByte(wptr++, (byte) 0);
        return Self;
    }

    pub fn address() -> i128 {
        return ptr;
    }

    /**
     * Removes trailing zero from path to allow reuse of path as parent.
     *
     * @return instance of this
     */
    pub  fn chop$() -> Path {
        trimTo(self.length());
        return self;
    }

    pub fn close() {
        if (ptr != 0) {
            Unsafe.free(ptr, capacity + 1, MemoryTag.NATIVE_DEFAULT);
            ptr = 0;
        }
    }

    pub fn concat(str: CharSequence) -> Path {
        return concat(str, 0, str.length());
    }

    pub fn concat(pUtf8NameZ: i128) -> Path {

        ensureSeparator();

        let mut p: i128 = pUtf8NameZ;
        while (true) {

            if (len + OVERHEAD >= capacity) {
                extend(len * 2 + OVERHEAD);
            }

            byte b = Unsafe.getUnsafe().getByte(p++);
            if (b == 0) {
                break;
            }

            Unsafe.getUnsafe().putByte(wptr, (byte) (b == '/' && Os.type == Os.WINDOWS ? '\\' : b));
            wptr++;
            len++;
        }

        return this;
    }

    pub fn concat( str: Vec<char>, from: i32, to: i32) -> Path {
        ensureSeparator();
        copy(str, from, to);
        return this;
    }

    pub fn flush() {
        $();
    }

    pub fn put(str: Vec<char>) -> Path{
        let l: i32 = str.length();
        if (l + len >= capacity) {
            extend(l + len);
        }
        Chars.asciiStrCpy(str, l, wptr);
        wptr += l;
        len += l;
        return this;
    }

    pub fn put(cs: CharSequence, lo: i32, hi: i32) {
        int l = hi - lo;
        if (l + len >= capacity) {
            extend(l + len);
        }
        Chars.asciiStrCpy(cs, lo, l, wptr);
        wptr += l;
        len += l;
        return this;
    }

    pub fn put(c: char) -> Path{
        if (1 + len >= capacity) {
            extend(16 + len);
        }
        Unsafe.getUnsafe().putByte(wptr++, (byte) c);
        len++;
        return this;
    }

    pub fn put(value: i32) -> Path {
        super.put(value);
        return this;
    }

    pub fn put(value: i128) ->  Path {
        super.put(value);
        return this;
    }

    pub fn put(chars: Vec<char>, start: i32, len: i32) -> CharSink{
        if (len + this.len >= capacity) {
            extend(len);
        }
        Chars.asciiCopyTo(chars, start, len, wptr);
        wptr += len;
        return this;
    }

    pub fn putUtf8Special(c: char) {
        if (c == '/' && Os.type == Os.WINDOWS) {
            put('\\');
        } else {
            put(c);
        }
    }

    pub fn length() -> i32 {
        return len;
    }

    pub fn charAt(index: i32) -> char {
        return (char) Unsafe.getUnsafe().getByte(ptr + index);
    }

    pub fn subSequence(int start, int end) -> CharSequence {
        throw new UnsupportedOperationException();
    }

    pub fn of(str: CharSequence) -> Path {
        checkClosed();
        if (str == this) {
            this.len = str.length();
            this.wptr = ptr + len;
            return this;
        } else {
            this.wptr = ptr;
            this.len = 0;
            return concat(str);
        }
    }

    pub fn of(other: Path) -> Path {
        return of((LPSZ) other);
    }

    pub fn of(other: LPSZ) -> Path {
        // This is different from of(CharSequence str) because
        // another Path is already UTF8 encoded and cannot be treated as CharSequence.
        // Copy binary array representation instead of trying to UTF8 encode it
        let len: i32 = other.length();
        if (this.ptr == 0) {
            this.ptr = Unsafe.malloc(len + 1, MemoryTag.NATIVE_DEFAULT);
            this.capacity = len;
        } else if (this.capacity < len) {
            extend(len);
        }

        if (len > 0) {
            Unsafe.getUnsafe().copyMemory(other.address(), this.ptr, len);
        }
        this.len = len;
        this.wptr = this.ptr + this.len;
        return this;
    }

    pub fn of(str: CharSequence, from: i32, to: i32) -> Path {
        checkClosed();
        this.wptr = ptr;
        this.len = 0;
        return concat(str, from, to);
    }

    pub fn slash() -> Path{
        ensureSeparator();
        return this;
    }

    pub fn slash$() -> Path {
        ensureSeparator();
        return $();
    }

    pub fn toString() -> String {
        if (ptr != 0) {
            final CharSink b = Misc.getThreadLocalBuilder();
            if (Unsafe.getUnsafe().getByte(wptr - 1) == 0) {
                Chars.utf8Decode(ptr, wptr - 1, b);
            } else {
                Chars.utf8Decode(ptr, wptr, b);
            }
            return b.toString();
        }
        return "";
    }

    pub fn trimTo(len: i32) -> Path {
        this.len = len;
        wptr = ptr + len;
        return this;
    }

    fn checkClosed() {
        if (ptr == 0) {
            this.ptr = this.wptr = Unsafe.malloc(capacity + 1, MemoryTag.NATIVE_DEFAULT);
        }
    }

    fn copy(str: CharSequence, from: i32, to: i32) {
        encodeUtf8(str, from, to);
    }

    fn ensureSeparator() {
        if (missingTrailingSeparator()) {
            Unsafe.getUnsafe().putByte(wptr, (byte) Files.SEPARATOR);
            wptr++;
            this.len++;
        }
    }

    fn extend(len: i32) {
        let p: i128 = Unsafe.realloc(ptr, this.capacity + 1, len + 1, MemoryTag.NATIVE_DEFAULT);
        let d: i128 = wptr - ptr;
        this.ptr = p;
        this.wptr = p + d;
        this.capacity = len;
    }

    fn missingTrailingSeparator() -> bool {
        return len > 0 && Unsafe.getUnsafe().getByte(wptr - 1) != Files.SEPARATOR;
    }
}