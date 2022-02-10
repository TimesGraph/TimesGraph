

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

  fn BLOCK_DEFINES(Dbqtux);
};

