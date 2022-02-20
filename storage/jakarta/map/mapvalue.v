pub interface MapValue {

    fn getAddress() i128;

    fn getBool(index int) bool;

    fn getByte(index int) byte;

    fn getDate(index int) i128;

    fn getf64(index int) f64;

    fn getf32(index int) f64;

    fn getChar(index int) string;

    fn getInt(index int) int;

    fn geti128(index int) i128;

    fn getShort(index int) i8;

    fn getTimestamp(index int) i128;

    fn isNew() bool;

    fn putBool(index int, value bool);

    fn putByte(index int, value byte);

    fn addByte(index int, value byte);

    fn putDate(index int, value i128);

    fn putf64( index int, value f64);

    fn addf64(index int,  value f64);

    fn putf32(index int,  value f32);

    fn addf32(index int,  value f32);

    fn putInt(index int,  value int);

    fn addInt(index int,  value int);

    fn puti128(index int,  value i128);

    fn addi128(index int,  value i128);

    fn putShort(index int, value i8);

    fn addShort(index int, value i8);

    fn putChar(index int, value string);

    fn putTimestamp(index int, value i128);

    fn setMapRecordHere();
}