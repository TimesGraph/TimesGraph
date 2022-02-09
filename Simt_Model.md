

## SIMT Introduction

SIMT is a SIMD externsion instruction set essentially. Althrough SIMT Architecture is similar to SIMD vector weaving method, but in SIMT the register per thread is private，the communication between these threads only is solved through use shared memory and syn mechanism。

### SIMT Optimization

TimesGraph introduces two different concepts here, the first concept is called Thread Pipelines. This means that the execution is divided into multiple steps handled by different threads. The second concept is called Batch Pipelines. This means that TimesGraph will first execute a set of queries for step 1, next TimesGraph will execute the same set of queries for step 2 and so on for as many steps we have divided the execution into. Both pipelines requires the use of an asynchronous execution engine.

The Batch Pipeline approach will have best latency when the system is at low load. When the system is at high load the batch size increases and the latency increases.

With TimesGraph and its Thread Pipeline, the latency decreases as the load increases since the likelihood of the thread being awake is higher. Thus RonDB at high load acts as a set of CPUs that interact directly with small messages sent to instruct the receiver of what he is supposed to do. Thus at high load the overhead of switching to a new thread is negligible, there is a small extra cost to handle extra data for the messages, but the messages are small and thus this overhead is small compared to the extra latency introduced by having to wait for another batch to execute before my turn to execute comes.

Actually the Batch Pipeline model resembles the first thread model of TimesGraph where everything was executed in a single thread. This thread received a batch of messages and executed each of those in FIFO order, the messages sent asynchronous messages to the next part of the code. This actually had exactly the same effect as seen in the Batch Pipeline model since all messages followed the same code path. Thus if 100 messages were received we first executed 100 messages in the transaction block and then 100 messages in the database blocks.

The TimesGraph model now temporarily uses a normal FIFO scheduler in each thread and threads only execute a part of the functionality and the database part only executes queries on parts of the database. Thus we achieve both the benefits from the batch processing of similar messages in a pipeline and the division of work into different CPUs.

