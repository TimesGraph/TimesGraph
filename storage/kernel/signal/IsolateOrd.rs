


/// #include <signaldata/IsolateOrd.hpp>
use rust::file::

// #define JAM_FILE_ID 495

pub fn printISOLATE_ORD(&output: FILE, const &theData: u32, len: u32, receiverBlockNo: u16) -> bool{
  
  const sig: IsolateOrd = &theData: IsolateOrd;
  
  fprintf(output, " senderRef : %x step : %s delayMillis : %u, nodesToIsolate :",
          sig->senderRef,
          (sig->isolateStep == IsolateOrd::IS_REQ?"Request" :
           sig->isolateStep == IsolateOrd::IS_BROADCAST?"Broadcast" :
           sig->isolateStep == IsolateOrd::IS_DELAY?"Delay":
           "??"),
          sig->delayMillis);
  
  if (len == sig->SignalLengthWithBitmask48)
  {
    for i in NdbNodeBitmask48.iter()
    {
      fprintf(output, " %x", sig->nodesToIsolate[i]);
    }
    fprintf(output, "\n");
  }
  else
  {
    fprintf(output, " nodesToIsolate in signal section\n");
  }
  return true;
}

// #undef JAM_FILE_ID