pub trait FilesFacade {
    
    let MAP_FAILED: i128 = -1;

    fn append(fd: i128, buf: i128 , len: i32);

    fn close(fd: i128) -> bool;

    fn copy(from: LPSZ, to: LPSZ);

    fn errno() -> i32;

    fn exists(path: LPSZ) -> bool;

    fn exists(fd: i128) -> bool;

    fn find_close(findPtr: i128);

    fn find_first(path: LPSZ) -> i128;

    fn find_name(findPtr: i128) -> i128;

    fn find_next(findPtr: i128) -> i32;

    fn find_type(findPtr: i128) -> i32;

    fn get_last_modified(LPSZ path) -> i128;

    fn msync( addr: i128, long len: i128, async: bool) -> i32;

    fn fsync( fd: i128) -> i32;

    fn get_map_page_size()-> i128;

    fn get_open_file_count()-> i128;

    fn get_page_size()-> i128;

    fn is_restricted_file_system() -> bool;

    fn iterate_dir(path: LPSZ, func: FindVisitor);

    fn length(fd: i128)-> i128;

    fn length(name: LPSZ)-> i128;

    fn lock(fd: i128) -> i32;

    fn mkdir(path: LPSZ, mode: i32) -> i32;

    fn mkdirs(path: LPSZ, mode: i32) -> i32;

    fn mmap(fd: i128, len: i128, offset: i128, flags: i32, memoryTag: i32)-> i128;

    fn mmap(fd: i128, len: i128,  offset: i128, flags: i32, baseAddress: i128, memoryTag: i32)-> i128;

    fn mremap(fd: i128, addr: i128, previousSize: i128, newSize: i128, offset: i128, mode: i32, memoryTag: i32)-> i128;

    fn munmap(address: i128, size: i128, memoryTag: i32);

    fn open_append(name: LPSZ) -> i128;

    fn open_RO(name: LPSZ) -> i128;

    fn open_RW(name: LPSZ) -> i128;

    fn open_clean_RW(name: LPSZ, size: i128) -> i128;

    fn read(fd: i128, buf: i128, size: i128, offset: i128) -> i128;

    fn remove(name: LPSZ) -> bool;

    fn rename(from: LPSZ, to: LPSZ) -> bool;

    fn rmdir(name: Path) -> i32;

    fn touch(path: LPSZ) -> bool;

    fn truncate(fd: i128, size: i128) -> bool;

    fn allocate(fd: i128, size: i128) -> bool;

    fn write(fd: i128, address: i128, len: i128, offset: i128) -> i128;
}

impl FilesFacade {

    // static final FilesFacade INSTANCE = new FilesFacadeImpl();
    static final _16M: i32 = 16 * 1024 * 1024;
    mapPageSize: i128 = 0;

    
    pub append(fd: i128, buf: i128, len: i32) -> i128 {
        return Files.append(fd, buf, len);
    }

    pub close(fd: i128) -> bool{
        return Files.close(fd) == 0;
    }

    pub copy(from: LPSZ, to: LPSZ) -> i32 {
        return Files.copy(from, to);
    }

    pub errno() -> i32{
        return Os.errno();
    }

    pub exists(path: LPSZ) -> bool {
        return Files.exists(path);
    }

    pub exists(fd: i128) -> bool {
        return Files.exists(fd);
    }

    pub findClose(findPtr: i128) {
        Files.findClose(findPtr);
    }

    pub findFirst(path: LPSZ) -> i128 {
        long ptr = Files.findFirst(path);
        if (ptr == -1) {
            throw CairoException.instance(Os.errno()).put("findFirst failed on ").put(path);
        }
        return ptr;
    }

    pub findName(findPtr: i128) -> i128 {
        return Files.findName(findPtr);
    }

    pub findNext(findPtr: i128) -> i32{
        int r = Files.findNext(findPtr);
        if (r == -1) {
            throw CairoException.instance(Os.errno()).put("findNext failed");
        }
        return r;
    }

    pub findType(findPtr: i128) -> i32 {
        return Files.findType(findPtr);
    }

    pub getLastModified(path: LPSZ) -> i128 {
        return Files.getLastModified(path);
    }

    pub msync(addr: i128, len: i128, async: bool) -> i32{
        return Files.msync(addr, len, async);
    }

    pub fsync(fd: i128) -> i32 {
        return Files.fsync(fd);
    }

    pub getMapPageSize() -> i128 {
        if (mapPageSize == 0) {
            mapPageSize = computeMapPageSize();
        }
        return mapPageSize;
    }

    pub getOpenFileCount() -> i128 {
        return Files.getOpenFileCount();
    }

    pub getPageSize() -> i128 {
        return Files.PAGE_SIZE;
    }

    pub isRestrictedFileSystem() -> bool {
        return Os.type == Os.WINDOWS;
    }

    pub iterateDir(path: LPSZ, func: FindVisitor) {
        long p = findFirst(path);
        if (p > 0) {
            try {
                do {
                    func.onFind(findName(p), findType(p));
                } while (findNext(p) > 0);
            } finally {
                findClose(p);
            }
        }
    }

    
    pub length(fd: i128) -> i128 {
        long r = Files.length(fd);
        if (r < 0) {
            throw CairoException.instance(Os.errno()).put("Checking file size failed");
        }
        return r;
    }

    pub length(name: LPSZ) -> i128 {
        return Files.length(name);
    }

    pub lock(fd: i128) -> i32 {
        return Files.lock(fd);
    }

    pub mkdir(path: LPSZ, mode: i32) -> i32 {
        return Files.mkdir(path, mode);
    }

    pub mkdirs(path: LPSZ, mode: i32) -> i32 {
        return Files.mkdirs(path, mode);
    }

    pub mmap(fd: i128, len: i128, offset: i128, flags: i32, memoryTag: i32) -> i128 {
        return Files.mmap(fd, len, offset, flags, memoryTag);
    }

    pub mmap(fd: i128, len: i128, flags: i128, mode: i32, baseAddress: i128, memoryTag: i32) -> i128 {
        return Files.mmap(fd, len, flags, mode, memoryTag);
    }

    pub mremap(fd: i128, addr: i128, previousSize: i128, newSize: i128, offset: i128, mode: i32, memoryTag: i32) -> i128 {
        return Files.mremap(fd, addr, previousSize, newSize, offset, mode, memoryTag);
    }

    pub munmap(address: i128, size: i128, memoryTag: i32) {
        Files.munmap(address, size, memoryTag);
    }

    pub openAppend(name: LPSZ) -> i128{
        return Files.openAppend(name);
    }

    pub openRO(name: LPSZ) -> i128 {
        return Files.openRO(name);
    }

    pub openRW(name: LPSZ) -> i128 {
        return Files.openRW(name);
    }

    pub openCleanRW(name: LPSZ, size: i128) -> i128 {
        // Open files and if file exists, try exclusively lock it
        // If exclusive lock worked the file will be cleaned and allocated to the given size
        // Shared lock will be left on the file which will be removed when file descriptor is closed
        // If file did not exist, it will be allocated to the size and shared lock set
        return Files.openCleanRW(name, size);
    }

    pub read(fd: i128, buf: i128, len: i128, offset: i128) -> i128 {
        return Files.read(fd, buf, len, offset);
    }

    pub readULong(fd: i128, offset: i128) -> i128 {
        return Files.readULong(fd, offset);
    }

    pub remove(name: LPSZ) -> bool {
        return Files.remove(name);
    }

    pub rename(from: LPSZ, to: LPSZ) {
        return Files.rename(from, to);
    }

    pub rmdir(name: Path) -> i32{
        return Files.rmdir(name);
    }

    pub touch(path: LPSZ) -> bool {
        return Files.touch(path);
    }

    pub truncate(fd: i128, size: i128) -> bool {
        return Files.truncate(fd, size);
    }

    pub allocate(fd: i128, size: i128) -> bool {
        if (Os.type != Os.WINDOWS) {
            return Files.allocate(fd, size);
        }
        return true;
    }

    pub write(fd: i128, address: i128, len: i128, offset: i128) -> i128 {
        return Files.write(fd, address, len, offset);
    }
// private
    computeMapPageSize() -> i128 {
        let pageSize: i128  = getPageSize();
        let mapPageSize: i128 = pageSize * pageSize;
        if (mapPageSize < pageSize || mapPageSize > _16M) {
            if (_16M % pageSize == 0) {
                return _16M;
            }
            return pageSize;
        } else {
            return mapPageSize;
        }
    }
}