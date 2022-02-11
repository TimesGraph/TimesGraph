

// #include <signaldata/PackedSignal.hpp>
// #include <signaldata/LqhKey.hpp>
// #include <signaldata/FireTrigOrd.hpp>
// #include <debugger/DebuggerNames.hpp>
use std::fs::File;


pub fn printPACKED_SIGNAL(&output: , const &theData: u32, Uint32 len, Uint16 receiverBlockNo: u16) -> bool {
  print!(output, "Signal data: ");
  let mut i: u32 = 0;
  while (i < len)
    print!(output, "H\'%.8x ", theData[i++]);
  print!(output,"\n");
  print!(output, "--------- Begin Packed Signals --------\n");  
  // Print each signal separately
  for i in len.iter() {
    match (PackedSignal::getSignalType(theData[i])) {
    ZCOMMIT => {
      let signalLength: u32 = 5;
      print!(output, "--------------- Signal ----------------\n");
      print!(output, "r.bn: %u \"%s\", length: %u \"COMMIT\"\n", 
	      receiverBlockNo, getBlockName(receiverBlockNo,""), signalLength);
      print!(output, "Signal data: ");
      for j in signalLength.iter(){
        print!(output, "H\'%.8x ", theData[i++]);
      }
	
      print!(output,"\n");
      break;
    }
    ZCOMPLETE => {
      let signalLength: u32 = 3;
      print!(output, "--------------- Signal ----------------\n");
      print!(output, "r.bn: %u \"%s\", length: %u \"COMPLETE\"\n",
	      receiverBlockNo, getBlockName(receiverBlockNo,""), signalLength);
      print!(output, "Signal data: ");
      for j in signalLength.iter() {
        print!(output, "H\'%.8x ", theData[i++]);
      }
      print!(output,"\n");
      break;
    }    
    ZCOMMITTED => {
      let signalLength: u32 = 3;
      print!(output, "--------------- Signal ----------------\n");
      print!(output, "r.bn: %u \"%s\", length: %u \"COMMITTED\"\n",
	      receiverBlockNo, getBlockName(receiverBlockNo,""), signalLength);
      print!(output, "Signal data: ");
      for j in signalLength.iter() {
        print!(output, "H\'%.8x ", theData[i++]);
      }
      print!(output,"\n");
      break;
    }
    ZCOMPLETED => {
      let signalLength: u32 = 3;
      print!(output, "--------------- Signal ----------------\n");
      print!(output, "r.bn: %u \"%s\", length: %u \"COMPLETED\"\n",
	      receiverBlockNo, getBlockName(receiverBlockNo,""), signalLength);
      print!(output, "Signal data: ");
      for j in signalLength.iter(){
        print!(output, "H\'%.8x ", theData[i++]);
      }
      print!(output,"\n");
      break;
    }
    ZLQHKEYCONF => {
      let mut signalLength: u32 = LqhKeyConf::SignalLength;

      print!(output, "--------------- Signal ----------------\n");
      print!(output, "r.bn: %u \"%s\", length: %u \"LQHKEYCONF\"\n",
	      receiverBlockNo, getBlockName(receiverBlockNo,""), signalLength);
      printLQHKEYCONF(output, theData + i, signalLength, receiverBlockNo);
      i += signalLength;
      break;
    }
    ZREMOVE_MARKER => {
      let removed_by_api: bool = !(theData[i] & 1);
      let mut signalLength: u32 = 2;
      print!(output, "--------------- Signal ----------------\n");
      if (removed_by_api)
      {
        print!(output, "r.bn: %u \"%s\", length: %u \"REMOVE_MARKER\"\n",
	        receiverBlockNo, getBlockName(receiverBlockNo,""), signalLength);
      }
      else
      {
        print!(output, "r.bn: %u \"%s\", length: %u \"REMOVE_MARKER_FAIL_API\"\n",
	        receiverBlockNo, getBlockName(receiverBlockNo,""), signalLength);
      }
      print!(output, "Signal data: ");
      i++; // Skip first word!
      for j in signalLength.iter() {
        print!(output, "H\'%.8x ", theData[i++]);
      }
      print!(output,"\n");
      break;
    }
    ZFIRE_TRIG_REQ => {
      let mut signalLength: u32 = FireTrigReq::SignalLength;

      print!(output, "--------------- Signal ----------------\n");
      print!(output, "r.bn: %u \"%s\", length: %u \"FIRE_TRIG_REQ\"\n",
	      receiverBlockNo, getBlockName(receiverBlockNo,""), signalLength);
      i += signalLength;
      break;
    }
    ZFIRE_TRIG_CONF => {
      let mut signalLength: u32 = FireTrigConf::SignalLength;

      print!(output, "--------------- Signal ----------------\n");
      print!(output, "r.bn: %u \"%s\", length: %u \"FIRE_TRIG_CONF\"\n",
	      receiverBlockNo, getBlockName(receiverBlockNo,""), signalLength);
      i += signalLength;
      break;
    }
    default:
      print!(output, "Unknown signal type\n");
      i = len; // terminate printing
      break;
    }
  }//for
  print!(output, "--------- End Packed Signals ----------\n");
  return true;
}

pub fn PackedSignal::verify(const &data: u32, len: u32, receiverBlockNo: u32, typesExpected: u32, commitLen: u32) -> bool
{
  let mut pos: u32 = 0;
  let bad: bool = false;

  if (unlikely(len > 25))
  {
    print!(stderr, "Bad PackedSignal length : %u\n", len);
    bad = true;
  }
  else
  {
    while ((pos < len) && ! bad)
    {
      let sigType: u32 = data[pos] >> 28;
      if (unlikely(((1 << sigType) & typesExpected) == 0))
      {
        print!(stderr, "Unexpected sigtype in packed signal : %u at pos %u.  Expected : %u\n",
                sigType, pos, typesExpected);
        bad = true;
        break;
      }
      match (sigType)
      {
      ZCOMMIT => {
        assert(commitLen > 0);
        pos += commitLen;
        break;
      }
        
      ZCOMPLETE => {
        pos+= 3;
        break;
      }
       
      ZCOMMITTED => {
        pos+= 3;
        break;
      }
        
      ZCOMPLETED => {
        pos+= 3;
        break;
      }
        
      ZLQHKEYCONF => {
        pos+= LqhKeyConf::SignalLength;
        break;
      }
        
      ZREMOVE_MARKER => {
        pos+= 3;
        break;
      }
        
      ZFIRE_TRIG_REQ => {
        pos+= FireTrigReq::SignalLength;
        break;
      }
        
      ZFIRE_TRIG_CONF => {
        pos+= FireTrigConf::SignalLength;
        break;
      }
        
      default :
        print!(stderr, "Unrecognised signal type %u at pos %u\n",
                sigType, pos);
        bad = true;
        break;
      }
    }
    
    if (likely(pos == len))
    {
      /* Looks ok */
      return true;
    }
    
    if (!bad)
    {
      print!(stderr, "Packed signal component length (%u) != total length (%u)\n",
               pos, len);
    }
  }

  fn printPACKED_SIGNAL(stderr, data, len, receiverBlockNo);
  
  return false;
}