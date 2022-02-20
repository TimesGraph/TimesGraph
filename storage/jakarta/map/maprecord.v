pub interface MapRecord {
    fn getValue() MapValue;

    fn setSymbolTableResolver(resolver RecordCursor, symbolTableIndex IntList);
}
