
/// #include <signaldata/TupKey.hpp>

fn printTUPKEYREQ(&output: FILE, const Uint32 * theData, Uint32 len, Uint16 receiverBlockNo) -> bool{
  fprintf(output, "Signal data: ");
  let i: u32 = 0;
  while (i < len)
    fprintf(output, "H\'%.8x ", theData[i++]);
  fprintf(output,"\n");
  
  return true;
}

fn printTUPKEYCONF(&output: FILE, const &theData: u32, Uint32 len, Uint16 receiverBlockNo) -> bool{
  fprintf(output, "Signal data: ");
  let i: u32 = 0;
  while (i < len)
    fprintf(output, "H\'%.8x ", theData[i++]);
  fprintf(output,"\n");
  
  return true;
}

fn printTUPKEYREF(&output: FILE, const &theData: u32, len: u32, receiverBlockNo: u16){
  fprintf(output, "Signal data: ");
  let i: u32 = 0;
  while (i < len)
    fprintf(output, "H\'%.8x ", theData[i++]);
  fprintf(output,"\n");
  
  return true;
}