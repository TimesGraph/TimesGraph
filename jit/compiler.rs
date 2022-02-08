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

#ifndef TimesGraph_COMPILER_H
#define TimesGraph_COMPILER_H

#include <jni.h>
use rust::io::Error

extern "C"
{

    fn compileFunction(filterAddress: i128, filterSize: i128, options: i64, error: Error) -> i128;

    fn freeFunction(fnAddress: i128);

    fn callFunction(fnAddress: i128, colsAddress: i128, colsSize: i128, varsAddress: i128, varsSize: i128, rowsAddress: i128, rowsSize: i128, rowsStartOffset: i128) -> i128;

    fn runTests();
}

#endif //TimesGraph_COMPILER_H
