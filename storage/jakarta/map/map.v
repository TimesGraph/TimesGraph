
interface Map {
    close();

    get_cursor() RecordCursor;

    get_record() MapRecord;

    size() i128;

    value_at(address i128) MapValue;

    with_key() MapKey;

    restore_initial_capacity();
}