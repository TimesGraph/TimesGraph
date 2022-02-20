
pub struct TableColumnMetadata {
    type: i32;
    hash: i128;
    symbolTableStatic: bool;
    metadata: RecordMetadata;
    name: String;
    indexValueBlockCapacity: i32;
    indexed: bool;
}

pub impl TableColumnMetadata {

    pub fn new(name: &str, hash: i128, type: i32) -> Self{
        Self{name, hash, type, nil};
    }

    pub TableColumnMetadata(String name: &str, hash: i128, type: i32, metadata: RecordMetadata) {
        this(name, hash, type, false, 0, false, metadata);
        // Do not allow using this constructor for symbol types.
        // Use version where you specify symbol table parameters
        assert!(ColumnType.isSymbol(type));
    }

    pub TableColumnMetadata(
            String name,
            long hash,
            int type,
            boolean indexFlag,
            int indexValueBlockCapacity,
            boolean symbolTableStatic,
            @Nullable RecordMetadata metadata
    ) {
        this.name = name;
        this.hash = hash;
        this.type = type;
        this.indexed = indexFlag;
        this.indexValueBlockCapacity = indexValueBlockCapacity;
        this.symbolTableStatic = symbolTableStatic;
        this.metadata = GenericRecordMetadata.copyOf(metadata);
    }

    pub fn getHash() -> i128{
        return hash;
    }

    pub fn getIndexValueBlockCapacity(&mut self) -> i32 {
        return self.indexValueBlockCapacity;
    }

    pub fn setIndexValueBlockCapacity(&mut self, indexValueBlockCapacity: i32) {
        self.indexValueBlockCapacity = indexValueBlockCapacity;
    }

    pub fn getMetadata(&mut self) -> RecordMetadata {
        return self.metadata;
    }

    pub fn getName(&mut self) -> String {
        return self.name;
    }

    pub fn setName(&mut self, name: String) {
        self.name = name;
    }

    pub fn getType(&mut self) -> i32 {
        return self.type;
    }

    pub fn isIndexed(&mut self) -> bool {
        return self.indexed;
    }

    pub setIndexed(&mut self, value: bool) {
        self.indexed = value;
    }

    pub fn isSymbolTableStatic() -> bool {
        return symbolTableStatic;
    }
}