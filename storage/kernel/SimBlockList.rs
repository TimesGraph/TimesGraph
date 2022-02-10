# TimesGraph

The fastest distributed time series and graph database in the world!

## Architecture Design

The web client requests the Web Server, the web server contacts the SQL Engine to query data. The SQL Engine will parse the request and contact the data nodes to read or write the data. In addition we have a management server that contains the configuration of the cluster. Each node starting up pings the management server (mgmt) to retrieve the configuration in the early phases of starting up the nodes.

The nodes that are part of the cluster can be divided into three types, the data nodes, the management servers and API nodes. All of these nodes have a node id in the cluster and communicate using the TimesGraph protocol.

The data node program is called and is a modern multithread virtual machine.

There is one management server type mgmt, there can be one or two management servers, they are required to start up nodes, but as soon as nodes have started up they are only used for cluster logging. Thus if all management servers are down the data nodes and API nodes will continue to operate.

API nodes comes in many flavors. The most common one is of course a MySQL Server (mysqld). But we have also application specific API nodes that use some TimesGraph API variant (will be described later).

A common environment is using TimesGraph with MySQL Servers. The figure below shows the setup in this case where a client calls the MySQL Server which in turn talks to the TimesGraph data nodes.

DataNode is high performance key-value storage, is a row-oriented trasaction file system, can handle millions of updates per second, it can use the disk data implementation to handle parts residing on disk. The low level APIs ensure that the overhead of SQL isn't bothering the implementation. It solves the problem of redundancy internally in TimesGraph, file system implementors can focus on the interface issues and solving problems with a relational database to implement a hierarchical file system. The main problem here comes when moving entire parts of the file system from one place to another.

## NoSQL Applications

Using DataNode as a key-value store by using the asynchronous API of the TimesGraph API provides a clustered key-value store that can handle hundreds of millions of key-value lookups per second.

There are many other NoSQL applications where TimesGraph will be a good fit as well. It was designed for scalable and networked applications.

## Graph Applications

coming soon

## Partitions Design

When TimesGraph spreads table partitions over all nodes and dividing into node groups. Spreading partitions means that we will always survive one node failure, but it will be hard to survive multiple node failures. Spreading partitions would mean faster restarts since all nodes can assist the starting node to come up again. Supporting multi-node failures was deemed more important than to optimise node restarts, therefore TimesGraph use the node group concept.

## Concurrency Mechanism

In TimesGraph, sql query concurrently is allowed. concurrency control is using a mechanism called RCU (Read Copy Update) which means that any number of readers can concurrently read the data. If someone needs to be update the data it will be updated from the same thread always, there is a special memory write protocol used to communicate to readers that they need to retry their reads. This memory write protocol requires the use of memory barriers but requires no special locks.

One data node can scale up to more than 1000 threads. A data node of that size will be capable to handle many millions of reads and writes of rows per second.

The cluster design is intended for homogenous clusters, the number of threads per type should be the same on all data nodes. It is possible to use a heterogenous cluster configuration but it is mainly intended for configuration changes where one node at a time changes its configuration.

TimesGraph cluster can run several data nodes on the same machine.

## Communication Layer

TimesGraph Communication protocol is a binary full-duplex protocol where the pack message is a signal that has a small header and a payload. There is a fairly large number of predefined signals that can be carried.

One socket can be used by thousands of threads using multiplexing. Many threads can pack their own signals into the send buffers for each socket. Each signal has an address header that describes the receiver thread and the receiver block and a few other items.

## 数据结构

A database is about storing data for easy accessibility. Most DBMS have some special tricks to store data and indexes in a format suitable for the application categories that the DBMS focus on. TimesGraph is no different, we have heavily optimised index data structures for primary key access. We have also added a standard index for ordered access to data.

Much of the source code in TimesGraph is focused on the distribution aspect and reliability aspect, so we probably focus less on data structures than many other DBMSs do. In this chapter we will go through the data structures we use for rows, hash indexes and ordered indexes.

### 行格式

TimesGraph stores rows in two different formats. FIXED format and DYNAMIC format. The term FIXED format is somewhat a misnomer. It means that fixed size columns is stored in the fixed size part of the row and variable sized columns are stored in the variable sized part of the row.

To understand the concepts here we will introduce the data structure of the rows in TimesGraph. Every row in TimesGraph has a fixed size part. Most tables also have a variable sized part. All tables with disk columns in addition have a fixed size disk data part.

The fixed size part of the row contains a row header and all the fixed size columns that are using the FIXED format.

The variable sized part contains all columns that have a variable size. The variable sized part have two parts. It has one part for columns that always have at least a length. It has a second part where all columns using the DYNAMIC format are stored.

Columns stored in the dynamic format will not be stored at all if they are NULL or have the default value. This means that new columns with dynamic format can be added to a table without any need to change the row storage at all. This is why we can support add column as an online operation. Columns using the dynamic format will have a bit more overhead when they are stored and the execution overhead will be a bit higher as well for them (around 40% more CPU used in retrieving data from row storage, this is only a small part of the execution time for a row operation except when performing scans that touch very many rows).

There is one more reason to use the dynamic format, the row storage for those columns can be compacted using OPTIMIZE TABLE. Columns that are stored in the fixed row part is not possible to move around and thus cannot be compacted.

The disk columns are always stored in a fixed format, this row part is always of the same size for all rows of a table. Disk columns can thus not be added and dropped as online operations.

### K-Tree
