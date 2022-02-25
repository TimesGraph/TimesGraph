
pub trait SkipList {

    pub keyOrder: KeyOrder[K]
  
    pub ranges: AtomicRanges[K] = AtomicRanges::new(keyOrder)
  
    pub nullKey: OK
    pub nullValue: OV
  
    fn put(key: K, value: V) -> Unit;

    fn put_if_absent(key: K, value: V) -> Boolean;

    fn get(key: K) -> OV;

    fn remove(key: K) -> Unit;
  
    fn lower(key: K) -> OV;

    fn lower_key(key: K)-> OK;
  
    fn floor(key: K) -> OV;

    fn floor_key_value(key: K) -> Option[(K, V)];
  
    fn higher(key: K) -> OV;

    fn higher_key(key: K) -> OK;

    fn higher_key_value(key: K) -> Option[(K, V)];
  
    fn ceiling(key: K) -> OV;

    fn ceiling_key(key: K) -> OK;
  
    fn is_empty -> Boolean;

    fn non_empty -> Boolean;

    fn clear() -> Unit;
    fn size -> Int;

    fn contains(key: K) -> Boolean;

    fn not_contains(key: K) -> Boolean = !contains(key);
  
    fn sub_map(from: K, fromInclusive: Boolean, to: K, toInclusive: Boolean) -> Iterable[(K, V)];
  
    fn subMapValues(from: K, fromInclusive: Boolean, to: K, toInclusive: Boolean) -> Iterable[V];
  
    fn headKey -> OK;

    fn lastKey -> OK;
  
    fn count() -> Int;
    
    fn last() -> OV;

    fn head() -> OV;

    fn headKeyValue -> Option[(K, V)];

    fn values() -> Iterable[V];

    fn foldLeft[R](r: R)(f: (R, (K, V)) => R) -> R;

    fn foreach[R](f: (K, V) => R) -> Unit;

    fn toIterable -> Iterable[(K, V)];

    fn iterator -> Iterator[(K, V)];

    fn valuesIterator -> Iterator[V];
  
    fn atomic_write[T, BAG[_]](from: K, to: K, toInclusive: Boolean)(f: => T)(implicit bag: Bag[BAG]) -> BAG[T] =
      AtomicRanges.writeAndRelease(from, to, toInclusive, f)(bag, ranges);
  
    fn atomicRead[BAG[_]](getKeys: V => (K, K, Boolean))(f: SkipList[OK, OV, K, V] => OV)(implicit bag: Bag[BAG]) -> BAG[OV] =
      AtomicRanges.readAndRelease(getKeys, nullValue, f(this))(bag, ranges);
    [inline]
    fn to_option_value(entry: Entry[K, V]) -> OV;
  
    [inline] 
    fn to_option_value(value: V) -> OV;
  
    [inline] 
    fn to_option_key(key: K) -> OK;

    [inline]
    fn to_option_key_value(entry: Entry[K, V]) -> Option[(K, V)];
      
    [inline]
    fn try_option_value(block: => Entry[K, V]) -> OV;
   
    [inline]
    fn try_option_key(block: => K) -> OK;
    [inline]
    fn try_option_key_value(block: => java.util.Map.Entry[K, V]) -> Option[(K, V)];
    
  }