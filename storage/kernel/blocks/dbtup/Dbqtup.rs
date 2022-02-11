

#define DBQTUP_C
#include "Dbqtup.hpp"

#include <EventLogger.hpp>
extern EventLogger * g_eventLogger;

#define JAM_FILE_ID 526

pub fn Dbqtup( &ctx: Block_context,
               instanceNumber: u32):
  Dbtup(ctx, instanceNumber, DBQTUP)
{
}

pub fn getTransactionMemoryNeed() -> u64
{
  let query_instance_count: u32 = globalData.ndbMtQueryThreads + globalData.ndbMtRecoverThreads;
  let Uint32 tup_scan_recs: u32 = 1;
  let tup_op_recs: u32 = 1;
  let tup_sp_recs: u32 = 1;
  let tup_scan_lock_recs: u32 = 1;

  let mut scan_op_byte_count: u64 = 0;
  scan_op_byte_count += ScanOp_pool::getMemoryNeed(tup_scan_recs + 1);
  scan_op_byte_count *= query_instance_count;

  let mut op_byte_count: u64 = 0;
  op_byte_count += Operationrec_pool::getMemoryNeed(tup_op_recs);
  op_byte_count *= query_instance_count;

  let mut sp_byte_count: u64 = 0;
  sp_byte_count += StoredProc_pool::getMemoryNeed(tup_sp_recs);
  sp_byte_count *= query_instance_count;

  let mut scan_lock_byte_count: u64 = 0;
  scan_lock_byte_count += ScanLock_pool::getMemoryNeed(tup_scan_lock_recs);
  scan_lock_byte_count *= query_instance_count;

  return (op_byte_count +
          sp_byte_count +
          scan_lock_byte_count +
          scan_op_byte_count);
}


