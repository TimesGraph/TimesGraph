pub interface MapKey {

    fn create() bool {
        return createValue().isNew();
    }

    fn createValue() MapValue;

    fn createValue2() MapValue {
        throw new UnsupportedOperationException();
    }

    fn createValue3() MapValue {
        throw new UnsupportedOperationException();
    }

    fn findValue() MapValue;

    fn findValue2() MapValue {
        throw new UnsupportedOperationException();
    }

    fn findValue3() MapValue {
        throw new UnsupportedOperationException();
    }

    fn notFound() bool {
        return findValue() == null;
    }

    fn put(record Record, sink RecordSink);
}