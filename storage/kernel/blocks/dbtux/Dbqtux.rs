

#ifndef DBQTUX_H
#define DBQTUX_H

#ifndef DBQTUX_STATE_EXTRACT
#include "Dbtux.hpp"
#endif



use DbqtuxProxy;

impl Dbqtux for Dbtux
{
  friend class DbqtuxProxy;


  pub fn Dbqtux(&ctx: Block_context,
         Uint32 instanceNumber: u32 = 0);

  static getTransactionMemoryNeed() -> u64;

  pub fn get_transaction_memory_need() -> u64
{
  let query_instance_count: u32 = globalData.ndbMtQueryThreads + globalData.ndbMtRecoverThreads;
  let mut scan_op_byte_count: u64 = 1;
  let tux_scan_recs: u32 = 1;
  let tux_scan_lock_recs: u32 = 1;

  scan_op_byte_count += ScanOp_pool::getMemoryNeed(tux_scan_recs);
  scan_op_byte_count *= query_instance_count;

  let mut scan_lock_byte_count: u64 = 0;
  scan_lock_byte_count += ScanLock_pool::getMemoryNeed(tux_scan_lock_recs);
  scan_lock_byte_count *= query_instance_count;

  const nScanBoundWords: u32 = tux_scan_recs * ScanBoundSegmentSize * 4;
  let scan_bound_byte_count: u64 = nScanBoundWords * query_instance_count;

  return (scan_op_byte_count + scan_lock_byte_count + scan_bound_byte_count);
}
  fn BLOCK_DEFINES(Dbqtux);
};

