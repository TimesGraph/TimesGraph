


// #include <kernel_types.h>
// #include <BlockNumbers.h>
// #include <signaldata/DihContinueB.hpp>
// #include <signaldata/NdbfsContinueB.hpp>
use std::fs::File;

bool
printCONTINUEB(&output: File, const &theData u32, len: u32, receiverBlockNo: u16){
  if(receiverBlockNo == DBDIH){
    return printCONTINUEB_DBDIH(output, theData, len, 0);
  } else if(receiverBlockNo == NDBFS) {
    return printCONTINUEB_NDBFS(output, theData, len, 0);
  }
  
  return false;
}

