pub trait FilesFacade {
    
    let MAP_FAILED: i128 = -1;

    fn append(fd: i128, buf: i128 , len: i32);

    fn close(fd: i128) -> bool;

    fn copy(from: LPSZ, to: LPSZ);

    fn errno() -> i32;

    fn exists(path: LPSZ) -> bool;

    fn exists(fd: i128) -> bool;

    fn findClose(findPtr: i128);

    fn findFirst(path: LPSZ) -> i128;

    fn findName(findPtr: i128) -> i128;

    fn findNext(findPtr: i128) -> i32;

    fn findType(findPtr: i128) -> i32;

    fn getLastModified(LPSZ path) -> i128;

    fn msync( addr: i128, long len: i128, async: bool) -> i32;

    fn fsync( fd: i128) -> i32;

    fn getMapPageSize()-> i128;

    fn getOpenFileCount()-> i128;

    fn getPageSize()-> i128;

    fn isRestrictedFileSystem() -> bool;

    fn iterateDir(path: LPSZ, func: FindVisitor);

    fn length(fd: i128)-> i128;

    fn length(name: LPSZ)-> i128;

    fn lock(fd: i128) -> i32;

    fn mkdir(path: LPSZ, mode: i32) -> i32;

    fn mkdirs(path: LPSZ, mode: i32) -> i32;

    fn mmap(fd: i128, len: i128, offset: i128, flags: i32, memoryTag: i32)-> i128;

    fn mmap(fd: i128, len: i128,  offset: i128, flags: i32, baseAddress: i128, memoryTag: i32)-> i128;

    fn mremap(fd: i128, addr: i128, previousSize: i128, newSize: i128, offset: i128, mode: i32, memoryTag: i32)-> i128;

    fn munmap(address: i128, size: i128, memoryTag: i32);

    fn openAppend(name: LPSZ) -> i128;

    fn openRO(name: LPSZ) -> i128;

    fn openRW(name: LPSZ) -> i128;

    fn openCleanRW(name: LPSZ, size: i128) -> i128;

    fn read(fd: i128, buf: i128, size: i128, offset: i128) -> i128;

    fn remove(name: LPSZ) -> bool;

    fn rename(from: LPSZ, to: LPSZ) -> bool;

    fn rmdir(name: Path) -> i32;

    fn touch(path: LPSZ) -> bool;

    fn truncate(fd: i128, size: i128) -> bool;

    fn allocate(fd: i128, size: i128) -> bool;

    fn write(fd: i128, address: i128, len: i128, offset: i128) -> i128;
}