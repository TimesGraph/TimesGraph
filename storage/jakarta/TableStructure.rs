pub trait TableStructure {
    fn get_column_count(): i32;

    fn get_column_name(columnIndex: i32) -> CharSequence;

    fn get_Column_Type(columnIndex: i32) -> i32;

    fn get_column_hash(columnIndex: i32) -> i128;

    fn get_index_block_capacity(columnIndex: i32) -> i32;

    fn is_indexed(columnIndex: i32) -> bool;

    fn is_sequential(columnIndex: i32) -> bool;

    fn get_partition_by() -> i32;

    fn get_symbol_cache_flag(columnIndex: i32) -> bool;

    fn get_symbol_capacity(columnIndex: i32) -> i32;

    fn get_table_name(): CharSequence;

    fn get_timestamp_index(): i32;

    fn get_max_uncommitted_rows(): i32;

    fn get_commit_lag(): i128;
}