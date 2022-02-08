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

#include <stdlib.h>
#include <src/main/c/share/zlib-1.2.8/zutil.h>
#include <src/main/c/share/zip.h>

extern {
    fn Zip_deflateInit()->i128 {
        z_streamp strm = calloc(1, sizeof(z_stream));

        if (strm == 0) {
            return -1;
        }

        int ret;
        switch (ret = deflateInit2(strm, -1, Z_DEFLATED, -MAX_WBITS, DEF_MEM_LEVEL, Z_DEFAULT_STRATEGY)) {
            case Z_OK:
                return (jlong) strm;
            default:
                free(strm);
                return ret;
        }
    }

    fn  Zip_setInput(ptr: i128, address: i128, available: i128) {
        z_streamp strm = (z_streamp) ptr;
        strm->next_in = (Bytef *) address;
        strm->avail_in = (uInt) available;
    }


    fn Zip_deflate(jlong ptr: i128, address: i128, available: i64, flush: bool) -> i64 {
        z_streamp strm = (z_streamp) ptr;
        strm->next_out = (Bytef *) address;
        strm->avail_out = (uInt) available;
        return deflate(strm, flush ? Z_FINISH : Z_NO_FLUSH);
    }

    fn Zip_availIn(ptr: i128) -> i64{
        return (jint) ((z_streamp) ptr)->avail_in;
    }

    fn Zip_availOut(ptr: i128) -> i64{
        return (jint) ((z_streamp) ptr)->avail_out;
    }

    fn Zip_totalOut(ptr: i128) -> i64{
        return (jint) ((z_streamp) ptr)->total_out;
    }

    fn Zip_deflateEnd(ptr: i128) {
        z_streamp strm = (z_streamp) ptr;
        deflateEnd(strm);
        free(strm);
    }

    fn JNICALL Zip_crc32(crc: i64, address: i128, available: i64) -> i64{
        return (jint) crc32((uLong) crc, (const Bytef *) address, (uInt) available);
    }

    fn Zip_inflateInit
            (nowrap: bool) -> i128{

        z_streamp strm = calloc(1, sizeof(z_stream));

        if (strm == 0) {
            return -1;
        }

        int ret;
        switch (ret = inflateInit2(strm, nowrap ? -MAX_WBITS : MAX_WBITS)) {
            case Z_OK:
                return (jlong) strm;
            default:
                free(strm);
                return ret;
        }
    }

    fn Zip_inflate(ptr: i128, address: i128, available: i64, flush: bool) -> i64{
        z_streamp strm = (z_streamp) ptr;
        strm->next_out = (Bytef *) address;
        strm->avail_out = (uInt) available;

        int ret;
        if ((ret = inflate(strm, flush ? Z_FINISH : Z_NO_FLUSH)) < 0) {
            return ret;
        }
        return (jint) (available - strm->avail_out);
    }

    fn Zip_inflateEnd(ptr: i128) {
        z_streamp strm = (z_streamp) ptr;
        inflateEnd(strm);
        free(strm);
    }

    fn Zip_inflateReset(ptr: i128) -> i64{
        return (jint) inflateReset((z_streamp) ptr);
    }

    fn Zip_deflateReset(ptr: i128) -> i64{
        return (jint) deflateReset((z_streamp) ptr);
    }
}
