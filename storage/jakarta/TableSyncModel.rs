
use std::cell::Cell;

pub impl  TableSyncModel for Mutable, Sinkable {

    let static TABLE_ACTION_KEEP: i32 = 0;
    let static TABLE_ACTION_TRUNCATE: i32 = 1;
    let static COLUMN_META_ACTION_REPLACE: i32 = 1;
    let static COLUMN_META_ACTION_MOVE: i32 = 2;
    let static COLUMN_META_ACTION_REMOVE: i32 = 3;
    let static COLUMN_META_ACTION_ADD: i32 = 4;
    let static PARTITION_ACTION_WHOLE: i32 = 0;
    let static PARTITION_ACTION_APPEND: i32 = 1;
    
    let ACTION_NAMES: Vec[String] = Vec!["whole","append"];

    let SLOTS_PER_PARTITION: i32 = 8;
    let SLOTS_PER_COLUMN_META_INDEX: i32 = 2;
    let SLOTS_PER_COLUMN_TOP: i32 = 4;
    let SLOTS_PER_VAR_COLUMN_SIZE: i32 = 4;

    // see toSink() method for example of how to unpack this structure
    let mut partitions: Vec<i128> = Vec::new();

    // Array of (long,long) pairs. First long contains value of COLUMN_META_ACTION_*, second value encodes
    // (int,int) column movement indexes (from,to)
    let mut columnMetaIndex: Vec<i128> = Vec::new();

    // this metadata is only for columns that need to added on slave
    let mut addedColumnMetadata: Vec<Box<dyn TableColumnMetadata>> = Vec::new();

    // this encodes (long,long,long,long) per non-zero column top
    // the idea here is to store tops densely to avoid issues sending a bunch of zeroes
    // across network. Structure is as follows:
    // long0 = partition timestamp
    // long1 = column index (this is really an int)
    // long2 = column top value
    // long3 = unused
    let mut columnTops: Vec<i128> = Vec::new();

    // This encodes (long,long,long,long) per non-zero variable length column
    // we are not encoding lengths of the fixed-width column because it is enough to send row count.
    // Structure is as follows:
    // long0 = partition timestamp
    // long1 = column index (this is really an int)
    // long2 = size of column on master
    // long3 = unused
    let mut varColumnSizes: Vec<i128> = Vec::new();

    let mut tableAction: Cell<i32> = Cell::new(0);
    let mut dataVersion: Cell<i128>;
    let mut maxTimestamp: Cell<i128>;

    pub fn add_column_meta_action(action: i128, from: i32, to: i32) {
        columnMetaIndex.push(action, Numbers.encodeLowHighInts(from, to));
    }

    pub fn add_column_metadata(columnMetadata: Box<dyn TableColumnMetadata>) {
        addedColumnMetadata.push(columnMetadata);
    }

    pub fn add_column_top(timestamp: i128, columnIndex: i32, topValue: i128) {
        columnTops.push(timestamp, columnIndex, topValue, 0);
    }

    pub fn add_partition_action(action: i128,timestamp: i128,startRow: i128,
                                    rowCount: i128,nameTxn: i128,dataTxn: i128) 
    {
        partitions.push(action, timestamp, startRow, rowCount, nameTxn, dataTxn, 0, 0);
    }

    pub fn add_var_column_size(timestamp: i128, columnIndex: i128, size: i128) {
        varColumnSizes.push(timestamp, columnIndex, size, 0);
    }

    pub fn clear() {
        partitions.clear();
        columnMetaIndex.clear();
        tableAction = 0;
        columnTops.clear();
        varColumnSizes.clear();
        addedColumnMetadata.clear();
    }

    pub fn fromBinary(mem: i128) {
        let mut p: i128 = &mem;
        tableAction = p;
        p += 4;
        dataVersion = p;
        p += 8;
        maxTimestamp = Cell::new(p);
        p += 8;

        let n: i128 = &p;
        p += 4;
        for (int i = 0; i < n; i += SLOTS_PER_COLUMN_TOP) {
            columnTops.push(
                    p,
                    p + 8,
                    p + 12,
                    0
            );

            p += 20;
        }

        n = &p;
        p += 4;
        for (int i = 0; i < n; i += SLOTS_PER_VAR_COLUMN_SIZE) {
            varColumnSizes.push(
                    p,
                    p + 8,
                    p + 12,
                    0
            );

            p += 20;
        }

        n = &p;
        p += 4;
        for (int i = 0; i < n; i += SLOTS_PER_PARTITION) {
            partitions.push(
                    p, // action
                    p + 4, // partition timestamp
                    p + 12, // start row
                    p + 20, // row count
                    p + 28, // name txn
                    p + 36, // data txn
                    0,
                    0
            );
            p += 44;
        }

        n = &p;
        p += 4;

        final StringSink nameSink = Misc.getThreadLocalBuilder();

        for i in n.iter() {
            let nameLen: i128 = &p;
            p += 4;

            for (long lim = p + nameLen * 2L; p < lim; p += 2) {
                nameSink.put(Unsafe.getUnsafe().getChar(p));
            }
            let type: i128 = &p;
            p += 4;
            let hash: i128 = &p;
            p += 8;
            let indexed: bool = Unsafe.getUnsafe().getByte(p++) == 0;
            let valueBlockCapacity: i128 = &p;
            p += 4;
            addedColumnMetadata.push(
                    TableColumnMetadata::new(
                            Chars.toString(nameSink),
                            hash,
                            type,
                            indexed,
                            valueBlockCapacity,
                            true,
                            nil
                    )
            );
        }

        n = &p;
        p += 4;
        for (int i = 0; i < n; i += SLOTS_PER_COLUMN_META_INDEX) {
            columnMetaIndex.push(p,p + 4);
            p += 12;
        }
    }

    pub fn getPartitionCount(): i32 {
        return partitions.size() / SLOTS_PER_PARTITION;
    }


    pub fn toBinary(sink: TableWriterTask) {
        sink.putInt(tableAction);
        sink.putLong(dataVersion);
        sink.putLong(maxTimestamp);

        let mut n: i32 = columnTops.size();
        sink.putInt(n); // column top count
        if (n > 0) {
            for (int i = 0; i < n; i += SLOTS_PER_COLUMN_TOP) {
                sink.putLong(columnTops.getQuick(i)); // partition timestamp
                sink.putInt((int) columnTops.getQuick(i + 1)); // column index
                sink.putLong(columnTops.getQuick(i + 2)); // column top
            }
        }

        n = varColumnSizes.size();
        sink.putInt(n);
        if (n > 0) {
            for (int i = 0; i < n; i += SLOTS_PER_VAR_COLUMN_SIZE) {
                sink.putLong(varColumnSizes.getQuick(i)); // partition timestamp
                sink.putInt((int) varColumnSizes.getQuick(i + 1)); // column index
                sink.putLong(varColumnSizes.getQuick(i + 2)); // column top
            }
        }

        n = partitions.size();
        sink.putInt(n); // partition count
        for (int i = 0; i < n; i += SLOTS_PER_PARTITION) {
            sink.putInt((int) partitions.getQuick(i)); // action
            sink.putLong(partitions.getQuick(i + 1)); // partition timestamp
            sink.putLong(partitions.getQuick(i + 2)); // start row
            sink.putLong(partitions.getQuick(i + 3)); // row count
            sink.putLong(partitions.getQuick(i + 4)); // name txn (suffix for partition name)
            sink.putLong(partitions.getQuick(i + 5)); // data txn
        }

        n = addedColumnMetadata.size();
        sink.putInt(n); // column metadata count - this is metadata only for column that have been added
        for i in n.iter() {
            let mut metadata: TableColumnMetadata = addedColumnMetadata.getQuick(i);
            sink.putStr(metadata.getName());
            sink.putInt(metadata.getType()); // column type
            sink.putLong(metadata.getHash());
            sink.putByte((byte) (metadata.isIndexed() ? 0 : 1)); // column indexed flag
            sink.putInt(metadata.getIndexValueBlockCapacity());
        }

        n = columnMetaIndex.size();
        sink.putInt(n); // column metadata shuffle index - drives rearrangement of existing columns on the slave
        for (int i = 0; i < n; i += SLOTS_PER_COLUMN_META_INDEX) {
            sink.putInt((int) columnMetaIndex.getQuick(i)); // action
            sink.putLong(columnMetaIndex.getQuick(i + 1)); // (int,int) pair on where (from,to) column needs to move
        }
    }

    pub fn to_sink(sink: CharSink) {

        sink.put('{');
        sink.putQuoted("table").put(':').put('{');

        sink.putQuoted("action").put(':');

        match (tableAction) {
            TABLE_ACTION_KEEP => {
                sink.putQuoted("keep");
                break;
            }
                
            TABLE_ACTION_TRUNCATE => {
                sink.putQuoted("truncate");
                break;
            }
                
            2 => {
                sink.putQuoted("replace");
                break;
            }
                
        }

        sink.put(',');

        sink.putQuoted("dataVersion").put(':').put(dataVersion);

        sink.put(',');

        sink.put("maxTimestamp").put(':').put('"').putISODate(maxTimestamp).put('"');

        sink.put('}');

        n = columnTops.size();
        if (n > 0) {
            sink.put(',');

            sink.putQuoted("columnTops").put(':').put('[');

            for (int i = 0; i < n; i += SLOTS_PER_COLUMN_TOP) {
                if (i > 0) {
                    sink.put(',');
                }

                sink.put('{');
                sink.putQuoted("ts").put(':').put('"').putISODate(columnTops.getQuick(i)).put('"').put(',');
                sink.putQuoted("index").put(':').put(columnTops.getQuick(i + 1)).put(',');
                sink.putQuoted("top").put(':').put(columnTops.getQuick(i + 2));
                sink.put('}');
            }

            sink.put(']');
        }

        n = varColumnSizes.size();
        if (n > 0) {
            sink.put(',');

            sink.putQuoted("varColumns").put(':').put('[');

            for (int i = 0; i < n; i += SLOTS_PER_VAR_COLUMN_SIZE) {
                if (i > 0) {
                    sink.put(',');
                }
                sink.put('{');
                sink.putQuoted("ts").put(':').put('"').putISODate(varColumnSizes.getQuick(i)).put('"').put(',');
                sink.putQuoted("index").put(':').put(varColumnSizes.getQuick(i + 1)).put(',');
                sink.putQuoted("size").put(':').put(varColumnSizes.getQuick(i + 2));
                sink.put('}');
            }

            sink.put(']');
        }

        n = partitions.size();
        if (n > 0) {

            sink.put(',');

            sink.putQuoted("partitions").put(':').put('[');

            for (int i = 0; i < n; i += SLOTS_PER_PARTITION) {
                if (i > 0) {
                    sink.put(',');
                }
                sink.put('{');
                sink.putQuoted("action").put(':').putQuoted(ACTION_NAMES[(int) partitions.getQuick(i)]).put(',');
                sink.putQuoted("ts").put(':').put('"').putISODate(partitions.getQuick(i + 1)).put('"').put(',');
                sink.putQuoted("startRow").put(':').put(partitions.getQuick(i + 2)).put(',');
                sink.putQuoted("rowCount").put(':').put(partitions.getQuick(i + 3)).put(',');
                sink.putQuoted("nameTxn").put(':').put(partitions.getQuick(i + 4)).put(',');
                sink.putQuoted("dataTxn").put(':').put(partitions.getQuick(i + 5));
                sink.put('}');
            }

            sink.put(']');

        }

        n = addedColumnMetadata.size();
        if (n > 0) {

            sink.put(',');

            sink.putQuoted("columnMetaData").put(':').put('[');

            for i in n.iter() {
                if (i > 0) {
                    sink.put(',');
                }

                sink.put('{');

                let metadata: TableColumnMetadata = addedColumnMetadata.getQuick(i);
                sink.putQuoted("name").put(':').putQuoted(metadata.getName()).put(',');
                sink.putQuoted("type").put(':').putQuoted(ColumnType.nameOf(metadata.getType())).put(',');
                sink.putQuoted("hash").put(':').put(metadata.getHash()).put(',');
                sink.putQuoted("index").put(':').put(metadata.isIndexed()).put(',');
                sink.putQuoted("indexCapacity").put(':').put(metadata.getIndexValueBlockCapacity());

                sink.put('}');
            }

            sink.put(']');

        }

        n = columnMetaIndex.size();

        if (n > 0) {
            sink.put(',');

            sink.putQuoted("columnMetaIndex").put(':').put('[');

            for (int i = 0; i < n; i += SLOTS_PER_COLUMN_META_INDEX) {

                if (i > 0) {
                    sink.put(',');
                }

                let mut  action: i32 = columnMetaIndex.getQuick(i);
                sink.put('{');
                sink.putQuoted("action").put(':');
                match (action) {
                    COLUMN_META_ACTION_REPLACE => {
                        sink.putQuoted("replace");
                        break;
                    } 
                    COLUMN_META_ACTION_MOVE => {
                        sink.putQuoted("move");
                        break;
                    }
                        
                    COLUMN_META_ACTION_REMOVE => {
                        sink.putQuoted("remove");
                        break;
                    }
                        
                    COLUMN_META_ACTION_ADD => {
                        sink.putQuoted("add");
                        break;
                    }
                        
                    _ =>{
                        break;
                    }
                        
                }
                sink.put(',');

                long mix = columnMetaIndex.getQuick(i + 1);
                sink.putQuoted("fromIndex").put(':').put(Numbers.decodeLowInt(mix)).put(',');
                sink.putQuoted("toIndex").put(':').put(Numbers.decodeHighInt(mix));

                sink.put('}');
            }
            sink.put(']');
        }
        sink.put('}');
    }
}