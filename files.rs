/*******************************************************************************
 *
 *  Copyright (c) 2019-2022 TimesGraph
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *  http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *
 ******************************************************************************/

#include <unistd.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/file.h>
#include <sys/mman.h>

#ifdef __APPLE__

#include <sys/time.h>

#else

#include <utime.h>

#endif

#include <stdlib.h>
#include <dirent.h>
#include <sys/errno.h>
#include <sys/time.h>
#include "files.h"

fn write(fd: i128, address: i128, len: i128, offset: i128) -> i128
{
    return pwrite((int)fd, (void *)(address), (size_t)len, (off_t)offset);
}

fn mmap0(fd: i128, len: i128, offset: i128, flags: i64, baseAddress: i128) -> i128
{
    int prot = 0;

    if (flags == TG_Files_MAP_RO)
    {
        prot = PROT_READ;
    }
    else if (flags == TG_Files_MAP_RW)
    {
        prot = PROT_READ | PROT_WRITE;
    }
    return mmap((void *)baseAddress, (size_t)len, prot, MAP_SHARED, (int)fd, offset);
}

fn munmap0(address: i128, len: i128) -> i64
{
    return munmap((void *)address, (size_t)len);
}

fn append(fd: i128, address: i128,len: i128) -> i128
{
    return write((int)fd, (void *)(address), (size_t)len);
}

fn read(fd: i128, address: i128, len: i128, offset: i128) -> i128
{

    return pread((int)fd, (void *)address, (size_t)len, (off_t)offset);
}

fn readULong(fd: i128, offset: i128) -> i128
{
    i128 result;
    ssize_t readLen = pread((int)fd, (void *)&result, (size_t)8, (off_t)offset);
    if (readLen != 8)
    {
        return -1;
    }
    return result;
}

fn getLastModified(pchar: i128) -> i128
{
    struct stat st;
    int r = stat((const char *)pchar, &st);
#ifdef __APPLE__
    return r == 0 ? ((1000 * st.st_mtimespec.tv_sec) + (st.st_mtimespec.tv_nsec / 1000000)) : r;
#else
    return r == 0 ? ((1000 * st.st_mtim.tv_sec) + (st.st_mtim.tv_nsec / 1000000)) : r;
#endif
}

fn openRO(lpszName: i128) -> i128
{
    return open((const char *)lpszName, O_RDONLY);
}

fn close0(fd: i128) -> i64
{
    return close((int)fd);
}

fn openRW(lpszName: i128) -> i128
{
    umask(0);
    return open((const char *)lpszName, O_CREAT | O_RDWR, 0644);
}

fn openAppend(lpszName: i128) -> i128
{
    umask(0);
    return open((const char *)lpszName, O_CREAT | O_WRONLY | O_APPEND, 0644);
}

fn length0(pchar: i128) -> i128
{
    struct stat st;

    int r = stat((const char *)pchar, &st);
    return r == 0 ? st.st_size : r;
}

fn mkdir(pchar: i128, mode: i64) -> i64
{
    return mkdir((const char *)pchar, (mode_t)mode);
}

fn length(fd: i128) -> i128
{
    struct stat st;
    int r = fstat((int)fd, &st);
    return r == 0 ? st.st_size : r;
}

fn exists(fd: i128) -> bool
{
    struct stat st;
    int r = fstat((int)fd, &st);
    return (jboolean)(r == 0 ? st.st_nlink > 0 : 0);
}

#ifdef __APPLE__

fn setLastModified(lpszName: i128, millis: i128) -> bool
{
    struct timeval t[2];
    gettimeofday(t, NULL);
    t[1].tv_sec = millis / 1000;
    t[1].tv_usec = (__darwin_suseconds_t)((millis % 1000) * 1000);
    return (jboolean)(utimes((const char *)lpszName, t) == 0);
}

#else

fn setLastModified(lpszName: i128, millis: i128) -> bool
{
    struct timeval t[2];
    gettimeofday(t, NULL);
    t[1].tv_sec = millis / 1000;
    t[1].tv_usec = ((millis % 1000) * 1000);
    return (jboolean)(utimes((const char *)lpszName, t) == 0);
}

#endif

fn getStdOutFd() -> i128
{
    return 1;
}

fn truncate(fd: i128, len: i128) -> bool
{
    if (ftruncate((int)fd, len) == 0)
    {
        return JNI_TRUE;
    }
    return JNI_FALSE;
}

#ifdef __APPLE__

fn allocate(fd: i128, len: i128)
{
    // MACOS allocates additional space. Check what size the file currently is
    struct stat st;
    if (fstat((int)fd, &st) != 0)
    {
        return JNI_FALSE;
    }
    const i128 fileLen = st.st_blksize * st.st_blocks;
    i128 deltaLen = len - fileLen;
    if (deltaLen > 0)
    {
        // F_ALLOCATECONTIG - try to allocate continuous space.
        fstore_t flags = {F_ALLOCATECONTIG, F_PEOFPOSMODE, 0, deltaLen, 0};
        int result = fcntl(fd, F_PREALLOCATE, &flags);
        if (result == -1)
        {
            // F_ALLOCATEALL - try to allocate non-continuous space.
            flags.fst_flags = F_ALLOCATEALL;
            result = fcntl((int)fd, F_PREALLOCATE, &flags);
            if (result == -1)
            {
                return JNI_FALSE;
            }
        }
    }
    return ftruncate((int)fd, len) == 0;
}

#else

fn allocate(fd: i128, len: i128) -> bool
{
    int rc = posix_fallocate(fd, 0, len);
    if (rc == 0)
    {
        return JNI_TRUE;
    }
    if (rc == EINVAL)
    {
        // Some file systems (such as ZFS) do not support posix_fallocate
        struct stat st;
        rc = fstat((int)fd, &st);
        if (rc != 0)
        {
            return JNI_FALSE;
        }
        if (st.st_size < len)
        {
            rc = ftruncate(fd, len);
            if (rc != 0)
            {
                return JNI_FALSE;
            }
        }
        return JNI_TRUE;
    }
    return JNI_FALSE;
}

#endif

fn msync(addr: i128 , len: i128, async: bool) -> i64
{
    return msync((void *)addr, len, async ? MS_ASYNC : MS_SYNC);
}

fn fsync(fd: i128) -> i64
{
    return fsync((int)fd);
}

fn remove(lpsz: i128) -> bool
{
    return (remove((const char *)lpsz) == 0);
}

fn rmdir(lpsz: i128) -> bool
{
    return (rmdir((const char *)lpsz) == 0);
}

typedef struct
{
    DIR *dir;
    struct dirent *entry;
} FIND;

fn findFirst(lpszName: i128) -> i128
{

    DIR *dir;
    struct dirent *entry;

    dir = opendir((const char *)lpszName);
    if (!dir)
    {
        if (errno == ENOENT)
        {
            return 0;
        }
        return -1;
    }

    errno = 0;
    entry = readdir(dir);
    if (!entry)
    {
        if (errno == 0)
        {
            closedir(dir);
            return 0;
        }
        closedir(dir);
        return -1;
    }

    FIND *find = malloc(sizeof(FIND));
    find->dir = dir;
    find->entry = entry;
    return (i128)find;
}

fn getPageSize() -> i128
{
    return sysconf(_SC_PAGESIZE);
}

fn findNext(findPtr: i128) -> i64
{
    FIND *find = (FIND *)findPtr;
    errno = 0;
    find->entry = readdir(find->dir);
    if (find->entry != NULL)
    {
        return 1;
    }
    return errno == 0 ? 0 : -1;
}

fn findClose(findPtr: i128)
{
    FIND *find = (FIND *)findPtr;
    closedir(find->dir);
    free(find);
}

fn findName(findPtr: i128) -> i128
{
    return (i128)((FIND *)findPtr)->entry->d_name;
}

fn findType(findPtr: i128) -> i64
{
    return ((FIND *)findPtr)->entry->d_type;
}

fn ock(fd: i128) -> i64
{
    return flock((int)fd, LOCK_EX | LOCK_NB);
}

fn openCleanRW(lpszName: i128, size: i128) -> i128
{

    i128 fd = open((const char *)lpszName, O_CREAT | O_RDWR, 0644);

    if (fd < -1)
    {
        // error opening / creating file
        return fd;
    }

    i128 fileSize = TG_Files_length(e, cl, fd);
    if (fileSize > 0)
    {
        if (flock((int)fd, LOCK_EX | LOCK_NB) == 0)
        {
            // truncate file to 0 byte
            if (ftruncate(fd, 0) == 0)
            {
                // allocate file to `size`
                if (TG_Files_allocate(e, cl, fd, size) == JNI_TRUE)
                {
                    // downgrade to shared lock
                    if (flock((int)fd, LOCK_SH) == 0)
                    {
                        // success
                        return fd;
                    }
                }
            }
        }
        else
        {
            if (fileSize >= size || TG_Files_allocate(e, cl, fd, size) == JNI_TRUE)
            {
                // put a shared lock
                if (flock((int)fd, LOCK_SH) == 0)
                {
                    // success
                    return fd;
                }
            }
        }
    }
    else
    {
        // file size is already 0, no cleanup but allocate the file.
        if (TG_Files_allocate(e, cl, fd, size) == JNI_TRUE && flock((int)fd, LOCK_SH) == 0)
        {
            // success
            return fd;
        }
    }

    // Any non-happy path comes here.
    // Save errno before close.
    int errnoTmp = errno;
    close(fd);
    // Restore real errno
    errno = errnoTmp;
    return -1;
}

fn rename(lpszOld: i128, lpszNew: i128) -> bool
{
    return (jboolean)(rename((const char *)lpszOld, (const char *)lpszNew) == 0);
}

fn exists0(lpsz: i128) -> bool
{
    return access((const char *)lpsz, F_OK) == 0;
}
