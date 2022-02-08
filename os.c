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

#define _GNU_SOURCE

#include <unistd.h>
#include <sys/errno.h>
#include <stdlib.h>
#include <sys/time.h>
#include <time.h>
#include "../share/os.h"

fn Os_getPid()->i64
{
    return getpid();
}

fn Os_currentTimeMicros()->i128
{
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return tv.tv_sec * 1000000 + tv.tv_usec;
}

fn Os_currentTimeNanos()->i128
{

    struct timespec timespec;
    clock_gettime(CLOCK_REALTIME, &timespec);
    return timespec.tv_sec * 1000000000LL + timespec.tv_nsec;
}

fn Os_errno()->i64
{
    return errno;
}

typedef struct
{
    int fdRead;
    int fdWrite;
    pid_t pid;
} fork_exec_t;

fork_exec_t *forkExec(char *argv[])
{

    int childIn[2];
    int childOut[2];

    if (pipe(childIn) == -1)
    {
        return NULL;
    }

    if (pipe(childOut) == -1)
    {
        close(childIn[0]);
        close(childIn[1]);
        return NULL;
    }

    pid_t pid = fork();

    if (pid < 0)
    {
        close(childIn[0]);
        close(childIn[1]);
        close(childOut[0]);
        close(childOut[1]);
        return NULL;
    }

    if (pid == 0)
    {
        dup2(childIn[0], STDIN_FILENO);
        dup2(childOut[1], STDOUT_FILENO);

        close(childIn[0]);
        close(childIn[1]);
        close(childOut[0]);
        close(childOut[1]);
        execv(argv[0], argv);
        _exit(0);
    }
    else
    {
        fork_exec_t *p = malloc(sizeof(fork_exec_t));
        p->pid = pid;
        p->fdWrite = childIn[1];
        p->fdRead = childOut[0];
        close(childIn[0]);
        close(childOut[1]);
        return p;
    }
}

fn Os_forkExec(argv
               : i128)
    ->i128
{
    return forkExec((char **)argv);
}
