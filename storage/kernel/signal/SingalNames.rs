/// #include <GlobalSignalNumbers.h>
use GSNName;

let mut SignalNames = HashMap::new();
SignalNames.insert("GSN_API_REGCONF","API_REGCONF");
SignalNames.insert("GSN_API_REGREF","API_REGREF");
SignalNames.insert("GSN_API_REGREQ","API_REGREQ");
SignalNames.insert("GSN_ATTRINFO","ATTRINFO");
SignalNames.insert( "GSN_SCHEMA_INFO","SCHEMA_INFO" );
SignalNames.insert( "GSN_SCHEMA_INFOCONF","SCHEMA_INFOCONF" );
SignalNames.insert( "GSN_GET_SCHEMA_INFOREQ","GET_SCHEMA_INFOREQ" );
SignalNames.insert( "GSN_DIHNDBTAMPER","DIHNDBTAMPER" );
SignalNames.insert( "GSN_KEYINFO",                "KEYINFO" );
SignalNames.insert( "GSN_KEYINFO20",              "KEYINFO20" );
SignalNames.insert( "GSN_KEYINFO20_R",            "KEYINFO20_R" );
SignalNames.insert( "GSN_NODE_FAILREP",           "NODE_FAILREP" );
SignalNames.insert( "GSN_READCONF",               "READCONF" );
SignalNames.insert( "GSN_SCAN_NEXTREQ",           "SCAN_NEXTREQ" );
SignalNames.insert( "GSN_SCAN_TABCONF",           "SCAN_TABCONF" );
SignalNames.insert( "GSN_SCAN_TABREF",            "SCAN_TABREF" );
SignalNames.insert( "GSN_SCAN_TABREQ",            "SCAN_TABREQ" );
SignalNames.insert( "GSN_TC_COMMITCONF",          "TC_COMMITCONF" );
SignalNames.insert( "GSN_TC_COMMITREF",           "TC_COMMITREF" );
SignalNames.insert( "GSN_TC_COMMITREQ",           "TC_COMMITREQ" );
SignalNames.insert( "GSN_TCKEY_FAILCONF",         "TCKEY_FAILCONF" );
SignalNames.insert( "GSN_TCKEY_FAILREF",          "TCKEY_FAILREF" );
SignalNames.insert( "GSN_TCKEYCONF",              "TCKEYCONF" );
SignalNames.insert( "GSN_TCKEYREF",               "TCKEYREF" );
SignalNames.insert( "GSN_TCKEYREQ",               "TCKEYREQ" );
SignalNames.insert( "GSN_TCRELEASECONF",          "TCRELEASECONF" );
  SignalNames.insert( "GSN_TCRELEASEREF",           "TCRELEASEREF" );
  SignalNames.insert( "GSN_TCRELEASEREQ",           "TCRELEASEREQ" );
  SignalNames.insert( "GSN_TCROLLBACKCONF",         "TCROLLBACKCONF" );
  SignalNames.insert( "GSN_TCROLLBACKREF",          "TCROLLBACKREF" );
  SignalNames.insert( "GSN_TCROLLBACKREQ",          "TCROLLBACKREQ" );
  SignalNames.insert( "GSN_TCROLLBACKREP",          "TCROLLBACKREP" );
  SignalNames.insert( "GSN_TCSEIZECONF",            "TCSEIZECONF" );
  SignalNames.insert( "GSN_TCSEIZEREF",             "TCSEIZEREF" );
  SignalNames.insert( "GSN_TCSEIZEREQ",             "TCSEIZEREQ" );
  SignalNames.insert( "GSN_DBINFO_SCANREQ",         "DBINFO_SCANREQ" );
  SignalNames.insert( "GSN_DBINFO_SCANCONF",        "DBINFO_SCANCONF" );
  SignalNames.insert( "GSN_DBINFO_SCANREF",         "DBINFO_SCANREF" );
  SignalNames.insert( "GSN_DBINFO_TRANSID_AI",      "DBINFO_TRANSID_AI" );
  SignalNames.insert( "GSN_TRANSID_AI",             "TRANSID_AI" );
  SignalNames.insert( "GSN_TRANSID_AI_R",           "TRANSID_AI_R" );
  SignalNames.insert( "GSN_ABORT",                  "ABORT" );
  SignalNames.insert( "GSN_ABORTCONF",              "ABORTCONF" );
  SignalNames.insert( "GSN_ABORTED",                "ABORTED" );
  SignalNames.insert( "GSN_ABORTREQ",               "ABORTREQ" );
  SignalNames.insert( "GSN_ACC_ABORTCONF",          "ACC_ABORTCONF" );
  SignalNames.insert( "GSN_ACC_ABORTREQ",           "ACC_ABORTREQ" );
  SignalNames.insert( "GSN_ACC_CHECK_SCAN",         "ACC_CHECK_SCAN" );
  SignalNames.insert( "GSN_ACC_COMMITCONF",         "ACC_COMMITCONF" );
  SignalNames.insert( "GSN_ACC_COMMITREQ",          "ACC_COMMITREQ" );
  SignalNames.insert( "GSN_ACC_OVER_REC",           "ACC_OVER_REC" );
  SignalNames.insert( "GSN_ACC_SCAN_INFO",          "ACC_SCAN_INFO" );
  SignalNames.insert( "GSN_ACC_SCANCONF",           "ACC_SCANCONF" );
  SignalNames.insert( "GSN_ACC_SCANREF",            "ACC_SCANREF" );
  SignalNames.insert( "GSN_ACC_SCANREQ",            "ACC_SCANREQ" );
  SignalNames.insert( "GSN_ACC_TO_CONF",            "ACC_TO_CONF" );
  SignalNames.insert( "GSN_ACC_TO_REF",             "ACC_TO_REF" );
  SignalNames.insert( "GSN_ACC_TO_REQ",             "ACC_TO_REQ" );
  SignalNames.insert( "GSN_ACCFRAGCONF",            "ACCFRAGCONF" );
  SignalNames.insert( "GSN_ACCFRAGREF",             "ACCFRAGREF" );
  SignalNames.insert( "GSN_ACCFRAGREQ",             "ACCFRAGREQ" );
  SignalNames.insert( "GSN_ACCKEYCONF",             "ACCKEYCONF" );
  SignalNames.insert( "GSN_ACCKEYREF",              "ACCKEYREF" );
  SignalNames.insert( "GSN_ACCKEYREQ",              "ACCKEYREQ" );
  SignalNames.insert( "GSN_ACCMINUPDATE",           "ACCMINUPDATE" );
  SignalNames.insert( "GSN_ACCSEIZECONF",           "ACCSEIZECONF" );
  SignalNames.insert( "GSN_ACCSEIZEREF",            "ACCSEIZEREF" );
  SignalNames.insert( "GSN_ACCSEIZEREQ",            "ACCSEIZEREQ" );
  SignalNames.insert( "GSN_ACCUPDATECONF",          "ACCUPDATECONF" );
  SignalNames.insert( "GSN_ACCUPDATEKEY",           "ACCUPDATEKEY" );
  SignalNames.insert( "GSN_ACCUPDATEREF",           "ACCUPDATEREF" );
  SignalNames.insert( "GSN_ADD_FRAGCONF",           "ADD_FRAGCONF" );
  SignalNames.insert( "GSN_ADD_FRAGREF",            "ADD_FRAGREF" );
  SignalNames.insert( "GSN_ADD_FRAGREQ",            "ADD_FRAGREQ" );
  SignalNames.insert( "GSN_API_START_REP",          "API_START_REP" );
  SignalNames.insert( "GSN_API_FAILCONF",           "API_FAILCONF" );
  SignalNames.insert( "GSN_API_FAILREQ",            "API_FAILREQ" );
  SignalNames.insert( "GSN_CHECK_LCP_STOP",         "CHECK_LCP_STOP" );
  SignalNames.insert( "GSN_CLOSE_COMCONF",          "CLOSE_COMCONF" );
  SignalNames.insert( "GSN_CLOSE_COMREQ",           "CLOSE_COMREQ" );
  SignalNames.insert( "GSN_CM_ACKADD",              "CM_ACKADD" );
  SignalNames.insert( "GSN_CM_ADD",                 "CM_ADD" );
  SignalNames.insert( "GSN_CM_ADD_REP",             "CM_ADD_REP" );  
  SignalNames.insert( "GSN_CM_HEARTBEAT",           "CM_HEARTBEAT" );
  SignalNames.insert( "GSN_CM_NODEINFOCONF",        "CM_NODEINFOCONF" );
  SignalNames.insert( "GSN_CM_NODEINFOREF",         "CM_NODEINFOREF" );
  SignalNames.insert( "GSN_CM_NODEINFOREQ",         "CM_NODEINFOREQ" );
  SignalNames.insert( "GSN_CM_REGCONF",             "CM_REGCONF" );
  SignalNames.insert( "GSN_CM_REGREF",              "CM_REGREF" );
  SignalNames.insert( "GSN_CM_REGREQ",              "CM_REGREQ" );
  SignalNames.insert( "GSN_CNTR_START_REQ",         "CNTR_START_REQ" );
  SignalNames.insert( "GSN_CNTR_START_REF",         "CNTR_START_REF" );
  SignalNames.insert( "GSN_CNTR_START_CONF",        "CNTR_START_CONF" );
  SignalNames.insert( "GSN_CNTR_START_REP",         "CNTR_START_REP" );
  SignalNames.insert( "GSN_CNTR_WAITREP",           "CNTR_WAITREP" );
  SignalNames.insert( "GSN_COMMIT",                 "COMMIT" );
  SignalNames.insert( "GSN_COMMIT_FAILCONF",        "COMMIT_FAILCONF" );
  SignalNames.insert( "GSN_COMMIT_FAILREQ",         "COMMIT_FAILREQ" );
  SignalNames.insert( "GSN_COMMITCONF",             "COMMITCONF" );
  SignalNames.insert( "GSN_COMMITREQ",              "COMMITREQ" );
  SignalNames.insert( "GSN_COMMITTED",              "COMMITTED" );
  SignalNames.insert( "GSN_LCP_FRAG_ORD",           "LCP_FRAG_ORD" );
  SignalNames.insert( "GSN_LCP_FRAG_REP",           "LCP_FRAG_REP" );
  SignalNames.insert( "GSN_LCP_COMPLETE_REP",       "LCP_COMPLETE_REP" );
  SignalNames.insert( "GSN_START_LCP_REQ",          "START_LCP_REQ" );
  SignalNames.insert( "GSN_START_LCP_CONF",         "START_LCP_CONF" );
  SignalNames.insert( "GSN_COMPLETE",               "COMPLETE" );
  SignalNames.insert( "GSN_COMPLETECONF",           "COMPLETECONF" );
  SignalNames.insert( "GSN_COMPLETED",              "COMPLETED" );
  SignalNames.insert( "GSN_COMPLETEREQ",            "COMPLETEREQ" );
  SignalNames.insert( "GSN_CONNECT_REP",            "CONNECT_REP" );
  SignalNames.insert( "GSN_CONTINUEB",              "CONTINUEB" );
  SignalNames.insert( "GSN_COPY_ACTIVECONF",        "COPY_ACTIVECONF" )
  SignalNames.insert( "GSN_COPY_ACTIVEREF",         "COPY_ACTIVEREF" )
  SignalNames.insert( "GSN_COPY_ACTIVEREQ",         "COPY_ACTIVEREQ" )
  SignalNames.insert( "GSN_COPY_FRAGCONF",          "COPY_FRAGCONF" )
  SignalNames.insert( "GSN_COPY_FRAGREF",           "COPY_FRAGREF" )
  SignalNames.insert( "GSN_COPY_FRAGREQ",           "COPY_FRAGREQ" )
  SignalNames.insert( "GSN_COPY_GCICONF",           "COPY_GCICONF" )
  SignalNames.insert( "GSN_COPY_GCIREQ",            "COPY_GCIREQ" )
  SignalNames.insert( "GSN_COPY_TABCONF",           "COPY_TABCONF" )
  SignalNames.insert( "GSN_COPY_TABREQ",            "COPY_TABREQ" )
  SignalNames.insert( "GSN_UPDATE_FRAG_STATECONF",  "UPDATE_FRAG_STATECONF" )
  SignalNames.insert( "GSN_UPDATE_FRAG_STATEREF",   "UPDATE_FRAG_STATEREF" )
  SignalNames.insert( "GSN_UPDATE_FRAG_STATEREQ",   "UPDATE_FRAG_STATEREQ" )
  SignalNames.insert( "GSN_DEBUG_SIG",              "DEBUG_SIG" )
  SignalNames.insert( "GSN_DIH_SCAN_TAB_REQ",       "DIH_SCAN_TAB_REQ" )
  SignalNames.insert( "GSN_DIH_SCAN_TAB_REF",       "DIH_SCAN_TAB_REF" )
  SignalNames.insert( "GSN_DIH_SCAN_TAB_CONF",      "DIH_SCAN_TAB_CONF" )
  SignalNames.insert( "GSN_DIH_SCAN_TAB_COMPLETE_REP", "DIH_SCAN_TAB_COMPLETE_REP" )
  SignalNames.insert( "GSN_DIADDTABCONF",           "DIADDTABCONF" )
  SignalNames.insert( "GSN_DIADDTABREF",            "DIADDTABREF" )
  SignalNames.insert( "GSN_DIADDTABREQ",            "DIADDTABREQ" )
  SignalNames.insert( "GSN_DICTSTARTCONF",          "DICTSTARTCONF" )
  SignalNames.insert( "GSN_DICTSTARTREQ",           "DICTSTARTREQ" )
  SignalNames.insert( "GSN_LIST_TABLES_REQ",        "LIST_TABLES_REQ" )
  SignalNames.insert( "GSN_LIST_TABLES_CONF",       "LIST_TABLES_CONF" )
  SignalNames.insert( "GSN_DIGETNODESCONF",         "DIGETNODESCONF" )
  SignalNames.insert( "GSN_DIGETNODESREF",          "DIGETNODESREF" )
  SignalNames.insert( "GSN_DIGETNODESREQ",          "DIGETNODESREQ" )
  SignalNames.insert( "GSN_DIH_RESTARTCONF",        "DIH_RESTARTCONF" )
  SignalNames.insert( "GSN_DIH_RESTARTREF",         "DIH_RESTARTREF" )
  SignalNames.insert( "GSN_DIH_RESTARTREQ",         "DIH_RESTARTREQ" )

  SignalNames.insert( "GSN_DISCONNECT_REP",         "DISCONNECT_REP" )
  SignalNames.insert( "GSN_DIVERIFYCONF",           "DIVERIFYCONF" )
  SignalNames.insert( "GSN_DIVERIFYREF",            "DIVERIFYREF" )
  SignalNames.insert( "GSN_DIVERIFYREQ",            "DIVERIFYREQ" )
  SignalNames.insert( "GSN_ENABLE_COMREQ",          "ENABLE_COMREQ" )
  SignalNames.insert( "GSN_ENABLE_COMCONF",         "ENABLE_COMCONF" )
  SignalNames.insert( "GSN_END_LCPCONF",            "END_LCPCONF" )
  SignalNames.insert( "GSN_END_LCPREQ",             "END_LCPREQ" )
  SignalNames.insert( "GSN_END_TOCONF",             "END_TOCONF" )
  SignalNames.insert( "GSN_END_TOREQ",              "END_TOREQ" )
  SignalNames.insert( "GSN_EVENT_REP",              "EVENT_REP" )
  SignalNames.insert( "GSN_EXEC_FRAGCONF",          "EXEC_FRAGCONF" )
  SignalNames.insert( "GSN_EXEC_FRAGREF",           "EXEC_FRAGREF" )
  SignalNames.insert( "GSN_EXEC_FRAGREQ",           "EXEC_FRAGREQ" )
  SignalNames.insert( "GSN_EXEC_SRCONF",            "EXEC_SRCONF" )
  SignalNames.insert( "GSN_EXEC_SRREQ",             "EXEC_SRREQ" )
  SignalNames.insert( "GSN_EXPANDCHECK2",           "EXPANDCHECK2" )
  SignalNames.insert( "GSN_FAIL_REP",               "FAIL_REP" )
  SignalNames.insert( "GSN_FSCLOSECONF",            "FSCLOSECONF" )
  SignalNames.insert( "GSN_FSCLOSEREF",             "FSCLOSEREF" )
  SignalNames.insert( "GSN_FSCLOSEREQ",             "FSCLOSEREQ" )
  SignalNames.insert( "GSN_FSOPENCONF",             "FSOPENCONF" )
  SignalNames.insert( "GSN_FSOPENREF",              "FSOPENREF" )
  SignalNames.insert( "GSN_FSOPENREQ",              "FSOPENREQ" )
  SignalNames.insert( "GSN_FSREADCONF",             "FSREADCONF" )
  SignalNames.insert( "GSN_FSREADREF",              "FSREADREF" )
  SignalNames.insert( "GSN_FSREADREQ",              "FSREADREQ" )
  SignalNames.insert( "GSN_FSSYNCCONF",             "FSSYNCCONF" )
  SignalNames.insert( "GSN_FSSYNCREF",              "FSSYNCREF" )
  SignalNames.insert( "GSN_FSSYNCREQ",              "FSSYNCREQ" )
  SignalNames.insert( "GSN_FSWRITECONF",            "FSWRITECONF" )
  SignalNames.insert( "GSN_FSWRITEREF",             "FSWRITEREF" )
  SignalNames.insert( "GSN_FSWRITEREQ",             "FSWRITEREQ" )
  SignalNames.insert( "GSN_FSAPPENDCONF",           "FSAPPENDCONF" )
  SignalNames.insert( "GSN_FSAPPENDREF",            "FSAPPENDREF" )
  SignalNames.insert( "GSN_FSAPPENDREQ",            "FSAPPENDREQ" )
  SignalNames.insert( "GSN_FSREMOVECONF",           "FSREMOVECONF" )
  SignalNames.insert( "GSN_FSREMOVEREF",            "FSREMOVEREF" )
  SignalNames.insert( "GSN_FSREMOVEREQ",            "FSREMOVEREQ" )
  SignalNames.insert( "GSN_GCP_ABORT",              "GCP_ABORT" )
  SignalNames.insert( "GSN_GCP_ABORTED",            "GCP_ABORTED" )
  SignalNames.insert( "GSN_GCP_COMMIT",             "GCP_COMMIT" )
  SignalNames.insert( "GSN_GCP_NODEFINISH",         "GCP_NODEFINISH" )
  SignalNames.insert( "GSN_GCP_NOMORETRANS",        "GCP_NOMORETRANS" )
  SignalNames.insert( "GSN_GCP_PREPARE",            "GCP_PREPARE" )
  SignalNames.insert( "GSN_GCP_PREPARECONF",        "GCP_PREPARECONF" )
  SignalNames.insert( "GSN_GCP_PREPAREREF",         "GCP_PREPAREREF" )
  SignalNames.insert( "GSN_GCP_SAVECONF",           "GCP_SAVECONF" )
  SignalNames.insert( "GSN_GCP_SAVEREF",            "GCP_SAVEREF" )
  SignalNames.insert( "GSN_GCP_SAVEREQ",            "GCP_SAVEREQ" )
  SignalNames.insert( "GSN_GCP_TCFINISHED",         "GCP_TCFINISHED" )
  SignalNames.insert( "GSN_GET_TABINFOREF",         "GET_TABINFOREF" )
  SignalNames.insert( "GSN_GET_TABINFOREQ",         "GET_TABINFOREQ" )
  SignalNames.insert( "GSN_GET_TABINFO_CONF",       "GET_TABINFO_CONF" )
  SignalNames.insert( "GSN_GETGCICONF",             "GETGCICONF" )
  SignalNames.insert( "GSN_GETGCIREQ",              "GETGCIREQ" )
  SignalNames.insert( "GSN_HOT_SPAREREP",           "HOT_SPAREREP" )
  SignalNames.insert( "GSN_INCL_NODECONF",          "INCL_NODECONF" )
  SignalNames.insert( "GSN_INCL_NODEREF",           "INCL_NODEREF" )
  SignalNames.insert( "GSN_INCL_NODEREQ",           "INCL_NODEREQ" )
  SignalNames.insert( "GSN_LQH_TRANSCONF",          "LQH_TRANSCONF" )
  SignalNames.insert( "GSN_LQH_TRANSREQ",           "LQH_TRANSREQ" )
  SignalNames.insert( "GSN_LQHADDATTCONF",          "LQHADDATTCONF" )
  SignalNames.insert( "GSN_LQHADDATTREF",           "LQHADDATTREF" )
  SignalNames.insert( "GSN_LQHADDATTREQ",           "LQHADDATTREQ" )
  SignalNames.insert( "GSN_LQHFRAGCONF",            "LQHFRAGCONF" )
  SignalNames.insert( "GSN_LQHFRAGREF",             "LQHFRAGREF" )
  SignalNames.insert( "GSN_LQHFRAGREQ",             "LQHFRAGREQ" )
  SignalNames.insert( "GSN_LQHKEYCONF",             "LQHKEYCONF" )
  SignalNames.insert( "GSN_LQHKEYREF",              "LQHKEYREF" )
  SignalNames.insert( "GSN_LQHKEYREQ",              "LQHKEYREQ" )
  SignalNames.insert( "GSN_MASTER_GCPCONF",         "MASTER_GCPCONF" )
  SignalNames.insert( "GSN_MASTER_GCPREF",          "MASTER_GCPREF" )
  SignalNames.insert( "GSN_MASTER_GCPREQ",          "MASTER_GCPREQ" )
  SignalNames.insert( "GSN_MASTER_LCPCONF",         "MASTER_LCPCONF" )
  SignalNames.insert( "GSN_MASTER_LCPREF",          "MASTER_LCPREF" )
  SignalNames.insert( "GSN_MASTER_LCPREQ",          "MASTER_LCPREQ" )
  SignalNames.insert( "GSN_MEMCHECKCONF",           "MEMCHECKCONF" )
  SignalNames.insert( "GSN_MEMCHECKREQ",            "MEMCHECKREQ" )
  SignalNames.insert( "GSN_NDB_FAILCONF",           "NDB_FAILCONF" )
  SignalNames.insert( "GSN_NDB_STARTCONF",          "NDB_STARTCONF" )
  SignalNames.insert( "GSN_NDB_STARTREF",           "NDB_STARTREF" )
  SignalNames.insert( "GSN_NDB_STARTREQ",           "NDB_STARTREQ" )
  SignalNames.insert( "GSN_NDB_STTOR",              "NDB_STTOR" )
  SignalNames.insert( "GSN_NDB_STTORRY",            "NDB_STTORRY" )
  SignalNames.insert( "GSN_NDB_TAMPER",             "NDB_TAMPER" )
  SignalNames.insert( "GSN_NEXT_SCANCONF",          "NEXT_SCANCONF" )
  SignalNames.insert( "GSN_NEXT_SCANREF",           "NEXT_SCANREF" )
  SignalNames.insert( "GSN_NEXT_SCANREQ",           "NEXT_SCANREQ" )
  SignalNames.insert( "GSN_NF_COMPLETEREP",         "NF_COMPLETEREP" )
  SignalNames.insert( "GSN_EXPAND_CLNT",            "EXPAND_CLNT" )
  SignalNames.insert( "GSN_OPEN_COMORD",            "OPEN_COMORD" )
  SignalNames.insert( "GSN_PACKED_SIGNAL",          "PACKED_SIGNAL" )
  SignalNames.insert( "GSN_PREP_FAILCONF",          "PREP_FAILCONF" )
  SignalNames.insert( "GSN_PREP_FAILREF",           "PREP_FAILREF" )
  SignalNames.insert( "GSN_PREP_FAILREQ",           "PREP_FAILREQ" )
  SignalNames.insert( "GSN_PRES_TOCONF",            "PRES_TOCONF" )
  SignalNames.insert( "GSN_PRES_TOREQ",             "PRES_TOREQ" )
  SignalNames.insert( "GSN_READ_NODESCONF",         "READ_NODESCONF" )
  SignalNames.insert( "GSN_READ_NODESREF",          "READ_NODESREF" )
  SignalNames.insert( "GSN_READ_NODESREQ",          "READ_NODESREQ" )
  SignalNames.insert( "GSN_SCAN_FRAGCONF",          "SCAN_FRAGCONF" )
  SignalNames.insert( "GSN_SCAN_FRAGREF",           "SCAN_FRAGREF" )
  SignalNames.insert( "GSN_SCAN_FRAGREQ",           "SCAN_FRAGREQ" )
  SignalNames.insert( "GSN_SCAN_HBREP",             "SCAN_HBREP" )
  SignalNames.insert( "GSN_SCAN_PROCCONF",          "SCAN_PROCCONF" )
  SignalNames.insert( "GSN_SCAN_PROCREQ",           "SCAN_PROCREQ" )
  SignalNames.insert( "GSN_SEND_PACKED",            "SEND_PACKED" )
  SignalNames.insert( "GSN_SET_LOGLEVELORD",        "SET_LOGLEVELORD" )
  SignalNames.insert( "GSN_SHRINKCHECK2",           "SHRINKCHECK2" )
  SignalNames.insert( "GSN_READ_CONFIG_REQ",        "READ_CONFIG_REQ" )
  SignalNames.insert( "GSN_READ_CONFIG_CONF",       "READ_CONFIG_CONF" )
  SignalNames.insert( "GSN_START_COPYCONF",         "START_COPYCONF" )
  SignalNames.insert( "GSN_START_COPYREF",          "START_COPYREF" )
  SignalNames.insert( "GSN_START_COPYREQ",          "START_COPYREQ" )
  SignalNames.insert( "GSN_START_EXEC_SR",          "START_EXEC_SR" )
  SignalNames.insert( "GSN_START_FRAGCONF",         "START_FRAGCONF" )
  SignalNames.insert( "GSN_START_FRAGREF",          "START_FRAGREF" )
  SignalNames.insert( "GSN_START_FRAGREQ",          "START_FRAGREQ" )
  SignalNames.insert( "GSN_START_LCP_REF",          "START_LCP_REF" )
  SignalNames.insert( "GSN_START_LCP_ROUND",        "START_LCP_ROUND" )
  SignalNames.insert( "GSN_START_MECONF",           "START_MECONF" )
  SignalNames.insert( "GSN_START_MEREF",            "START_MEREF" )
  SignalNames.insert( "GSN_START_MEREQ",            "START_MEREQ" )
  SignalNames.insert( "GSN_START_PERMCONF",         "START_PERMCONF" )
  SignalNames.insert( "GSN_START_PERMREF",          "START_PERMREF" )
  SignalNames.insert( "GSN_START_PERMREQ",          "START_PERMREQ" )
  SignalNames.insert( "GSN_START_RECCONF",          "START_RECCONF" )
  SignalNames.insert( "GSN_START_RECREF",           "START_RECREF" )
  SignalNames.insert( "GSN_START_RECREQ",           "START_RECREQ" )
  SignalNames.insert( "GSN_START_TOCONF",           "START_TOCONF" )
  SignalNames.insert( "GSN_START_TOREQ",            "START_TOREQ" )
  SignalNames.insert( "GSN_STORED_PROCCONF",        "STORED_PROCCONF" )
  SignalNames.insert( "GSN_STORED_PROCREF",         "STORED_PROCREF" )
  SignalNames.insert( "GSN_STORED_PROCREQ",         "STORED_PROCREQ" )
  SignalNames.insert( "GSN_STTOR",                  "STTOR" )
  SignalNames.insert( "GSN_STTORRY",                "STTORRY" )
  SignalNames.insert( "GSN_SYSTEM_ERROR",           "SYSTEM_ERROR" )
  SignalNames.insert( "GSN_TAB_COMMITCONF",         "TAB_COMMITCONF" )
  SignalNames.insert( "GSN_TAB_COMMITREF",          "TAB_COMMITREF" )
  SignalNames.insert( "GSN_TAB_COMMITREQ",          "TAB_COMMITREQ" )
  SignalNames.insert( "GSN_TAKE_OVERTCCONF",        "TAKE_OVERTCCONF" )
  SignalNames.insert( "GSN_TAKE_OVERTCREQ",         "TAKE_OVERTCREQ" )
  SignalNames.insert( "GSN_TC_CLOPSIZECONF",        "TC_CLOPSIZECONF" )
  SignalNames.insert( "GSN_TC_CLOPSIZEREQ",         "TC_CLOPSIZEREQ" )
  SignalNames.insert( "GSN_TC_SCHVERCONF",          "TC_SCHVERCONF" )
  SignalNames.insert( "GSN_TC_SCHVERREQ",           "TC_SCHVERREQ" )
  SignalNames.insert( "GSN_TCGETOPSIZECONF",        "TCGETOPSIZECONF" )
  SignalNames.insert( "GSN_TCGETOPSIZEREQ",         "TCGETOPSIZEREQ" )
  SignalNames.insert( "GSN_TEST_ORD",               "TEST_ORD" )
  SignalNames.insert( "GSN_TESTSIG",                "TESTSIG" )
  SignalNames.insert( "GSN_TIME_SIGNAL",            "TIME_SIGNAL" )
  SignalNames.insert( "GSN_TUP_ABORTREQ",           "TUP_ABORTREQ" )
  SignalNames.insert( "GSN_TUP_ADD_ATTCONF",        "TUP_ADD_ATTCONF" )
  SignalNames.insert( "GSN_TUP_ADD_ATTRREF",        "TUP_ADD_ATTRREF" )
  SignalNames.insert( "GSN_TUP_ADD_ATTRREQ",        "TUP_ADD_ATTRREQ" )
  SignalNames.insert( "GSN_TUP_ATTRINFO",           "TUP_ATTRINFO" )
  SignalNames.insert( "GSN_TUP_COMMITREQ",          "TUP_COMMITREQ" )
  SignalNames.insert( "GSN_TUPFRAGCONF",            "TUPFRAGCONF" )
  SignalNames.insert( "GSN_TUPFRAGREF",             "TUPFRAGREF" )
  SignalNames.insert( "GSN_TUPFRAGREQ",             "TUPFRAGREQ" )
  SignalNames.insert( "GSN_TUPKEYCONF",             "TUPKEYCONF" )
  SignalNames.insert( "GSN_TUPKEYREF",              "TUPKEYREF" )
  SignalNames.insert( "GSN_TUPKEYREQ",              "TUPKEYREQ" )
  SignalNames.insert( "GSN_TUPRELEASECONF",         "TUPRELEASECONF" )
  SignalNames.insert( "GSN_TUPRELEASEREF",          "TUPRELEASEREF" )
  SignalNames.insert( "GSN_TUPRELEASEREQ",          "TUPRELEASEREQ" )
  SignalNames.insert( "GSN_TUPSEIZECONF",           "TUPSEIZECONF" )
  SignalNames.insert( "GSN_TUPSEIZEREF",            "TUPSEIZEREF" )
  SignalNames.insert( "GSN_TUPSEIZEREQ",            "TUPSEIZEREQ" )
  SignalNames.insert( "GSN_UNBLO_DICTCONF",         "UNBLO_DICTCONF" )
  SignalNames.insert( "GSN_UNBLO_DICTREQ",          "UNBLO_DICTREQ" )
  SignalNames.insert( "GSN_UPDATE_TOCONF",          "UPDATE_TOCONF" )
  SignalNames.insert( "GSN_UPDATE_TOREF",           "UPDATE_TOREF" )
  SignalNames.insert( "GSN_UPDATE_TOREQ",           "UPDATE_TOREQ" )
  SignalNames.insert( "GSN_TUP_DEALLOCREQ",         "TUP_DEALLOCREQ" )
  SignalNames.insert( "GSN_TUP_WRITELOG_REQ",       "TUP_WRITELOG_REQ" )
  SignalNames.insert( "GSN_LQH_WRITELOG_REQ",       "LQH_WRITELOG_REQ" )

  SignalNames.insert( "GSN_START_ORD",              "START_ORD" )
  SignalNames.insert( "GSN_STOP_ORD",               "STOP_ORD" )
  SignalNames.insert( "GSN_TAMPER_ORD",             "TAMPER_ORD" )

  SignalNames.insert( "GSN_EVENT_SUBSCRIBE_REQ",    "EVENT_SUBSCRIBE_REQ" )
  SignalNames.insert( "GSN_EVENT_SUBSCRIBE_CONF",   "EVENT_SUBSCRIBE_CONF" )
  SignalNames.insert( "GSN_EVENT_SUBSCRIBE_REF",    "EVENT_SUBSCRIBE_REF" )
  SignalNames.insert( "GSN_DUMP_STATE_ORD",         "DUMP_STATE_ORD" )

  SignalNames.insert( "GSN_NODE_START_REP", "NODE_START_REP" )

  SignalNames.insert( "GSN_START_INFOREQ",  "START_INFOREQ" )
  SignalNames.insert( "GSN_START_INFOREF",  "START_INFOREF" )
  SignalNames.insert( "GSN_START_INFOCONF", "START_INFOCONF" )

  SignalNames.insert( "GSN_CHECKNODEGROUPSREQ",     "CHECKNODEGROUPSREQ" )
  SignalNames.insert( "GSN_CHECKNODEGROUPSCONF",    "CHECKNODEGROUPSCONF" )

  SignalNames.insert( "GSN_ARBIT_PREPREQ",          "ARBIT_PREPREQ" )
  SignalNames.insert( "GSN_ARBIT_PREPCONF",         "ARBIT_PREPCONF" )
  SignalNames.insert( "GSN_ARBIT_PREPREF",          "ARBIT_PREPREF" )
  SignalNames.insert( "GSN_ARBIT_STARTREQ",         "ARBIT_STARTREQ" )
  SignalNames.insert( "GSN_ARBIT_STARTCONF",        "ARBIT_STARTCONF" )
  SignalNames.insert( "GSN_ARBIT_STARTREF",         "ARBIT_STARTREF" )
  SignalNames.insert( "GSN_ARBIT_CHOOSEREQ",        "ARBIT_CHOOSEREQ" )
  SignalNames.insert( "GSN_ARBIT_CHOOSECONF",       "ARBIT_CHOOSECONF" )
  SignalNames.insert( "GSN_ARBIT_CHOOSEREF",        "ARBIT_CHOOSEREF" )
  SignalNames.insert( "GSN_ARBIT_STOPORD",          "ARBIT_STOPORD" )
  SignalNames.insert( "GSN_ARBIT_STOPREP",          "ARBIT_STOPREP" )

  SignalNames.insert( "GSN_TC_COMMIT_ACK",          "TC_COMMIT_ACK" )
  SignalNames.insert( "GSN_REMOVE_MARKER_ORD",      "REMOVE_MARKER_ORD" )

  SignalNames.insert( "GSN_NODE_STATE_REP",         "NODE_STATE_REP" )
  SignalNames.insert( "GSN_CHANGE_NODE_STATE_REQ",  "CHANGE_NODE_STATE_REQ" )
  SignalNames.insert( "GSN_CHANGE_NODE_STATE_CONF", "CHANGE_NODE_STATE_CONF" )

  SignalNames.insert( "GSN_BLOCK_COMMIT_ORD",       "BLOCK_COMMIT_ORD" )
  SignalNames.insert( "GSN_UNBLOCK_COMMIT_ORD",     "UNBLOCK_COMMIT_ORD" )
  
  SignalNames.insert( "GSN_DIH_SWITCH_REPLICA_REQ",  "DIH_SWITCH_REPLICA_REQ" )
  SignalNames.insert( "GSN_DIH_SWITCH_REPLICA_REF",  "DIH_SWITCH_REPLICA_REF" )
  SignalNames.insert( "GSN_DIH_SWITCH_REPLICA_CONF", "DIH_SWITCH_REPLICA_CONF" )
  
  SignalNames.insert( "GSN_STOP_PERM_REQ",           "STOP_PERM_REQ" )
  SignalNames.insert( "GSN_STOP_PERM_REF",           "STOP_PERM_REF" )
  SignalNames.insert( "GSN_STOP_PERM_CONF",          "STOP_PERM_CONF" )

  SignalNames.insert( "GSN_STOP_ME_REQ",             "STOP_ME_REQ" )
  SignalNames.insert( "GSN_STOP_ME_REF",             "STOP_ME_REF" )
  SignalNames.insert( "GSN_STOP_ME_CONF",            "STOP_ME_CONF" )

  SignalNames.insert( "GSN_WAIT_GCP_REQ",           "WAIT_GCP_REQ" )
  SignalNames.insert( "GSN_WAIT_GCP_REF",           "WAIT_GCP_REF" )
  SignalNames.insert( "GSN_WAIT_GCP_CONF",          "WAIT_GCP_CONF" )

  SignalNames.insert( "GSN_STOP_REQ",               "STOP_REQ" )
  SignalNames.insert( "GSN_STOP_REF",               "STOP_REF" )
  SignalNames.insert( "GSN_API_VERSION_REQ",        "API_VERSION_REQ" )
  SignalNames.insert( "GSN_API_VERSION_CONF",       "API_VERSION_CONF" )

  SignalNames.insert( "GSN_ABORT_ALL_REQ",          "ABORT_ALL_REQ" )
  SignalNames.insert( "GSN_ABORT_ALL_REF",          "ABORT_ALL_REF" )
  SignalNames.insert( "GSN_ABORT_ALL_CONF",         "ABORT_ALL_CONF" )

  SignalNames.insert( "GSN_DROP_TABLE_REQ",         "DROP_TABLE_REQ" )
  SignalNames.insert( "GSN_DROP_TABLE_REF",         "DROP_TABLE_REF" )
  SignalNames.insert( "GSN_DROP_TABLE_CONF",        "DROP_TABLE_CONF" )

  SignalNames.insert( "GSN_DROP_TAB_REQ",           "DROP_TAB_REQ" )
  SignalNames.insert( "GSN_DROP_TAB_REF",           "DROP_TAB_REF" )
  SignalNames.insert( "GSN_DROP_TAB_CONF",          "DROP_TAB_CONF" )
  
  SignalNames.insert( "GSN_PREP_DROP_TAB_REQ",      "PREP_DROP_TAB_REQ" )
  SignalNames.insert( "GSN_PREP_DROP_TAB_REF",      "PREP_DROP_TAB_REF" )
  SignalNames.insert( "GSN_PREP_DROP_TAB_CONF",     "PREP_DROP_TAB_CONF" )

  SignalNames.insert( "GSN_CREATE_TRIG_REQ",        "CREATE_TRIG_REQ" )
  SignalNames.insert( "GSN_CREATE_TRIG_CONF",       "CREATE_TRIG_CONF" )
  SignalNames.insert( "GSN_CREATE_TRIG_REF",        "CREATE_TRIG_REF" )
  SignalNames.insert( "GSN_ALTER_TRIG_REQ",         "ALTER_TRIG_REQ" )
  SignalNames.insert( "GSN_ALTER_TRIG_CONF",        "ALTER_TRIG_CONF" )
  SignalNames.insert( "GSN_ALTER_TRIG_REF",         "ALTER_TRIG_REF" )
  SignalNames.insert( "GSN_DROP_TRIG_REQ",          "DROP_TRIG_REQ" )
  SignalNames.insert( "GSN_DROP_TRIG_CONF",         "DROP_TRIG_CONF" )
  SignalNames.insert( "GSN_DROP_TRIG_REF",          "DROP_TRIG_REF" )
  SignalNames.insert( "GSN_FIRE_TRIG_ORD",          "FIRE_TRIG_ORD" )
  SignalNames.insert( "GSN_FIRE_TRIG_ORD_L",        "FIRE_TRIG_ORD_L" )
  SignalNames.insert( "GSN_TRIG_ATTRINFO",          "TRIG_ATTRINFO" )

  SignalNames.insert( "GSN_CREATE_INDX_REQ",        "CREATE_INDX_REQ" )
  SignalNames.insert( "GSN_CREATE_INDX_CONF",       "CREATE_INDX_CONF" )
  SignalNames.insert( "GSN_CREATE_INDX_REF",        "CREATE_INDX_REF" )
  SignalNames.insert( "GSN_DROP_INDX_REQ",          "DROP_INDX_REQ" )
  SignalNames.insert( "GSN_DROP_INDX_CONF",         "DROP_INDX_CONF" )
  SignalNames.insert( "GSN_DROP_INDX_REF",          "DROP_INDX_REF" )
  SignalNames.insert( "GSN_ALTER_INDX_REQ",         "ALTER_INDX_REQ" )
  SignalNames.insert( "GSN_ALTER_INDX_CONF",        "ALTER_INDX_CONF" )
  SignalNames.insert( "GSN_ALTER_INDX_REF",         "ALTER_INDX_REF" )
  SignalNames.insert( "GSN_TCINDXREQ", 		"TCINDXREQ" )
  SignalNames.insert( "GSN_TCINDXCONF", 		"TCINDXCONF" )
  SignalNames.insert( "GSN_TCINDXREF", 		"TCINDXREF" )
  SignalNames.insert( "GSN_INDXKEYINFO", 		"INDXKEYINFO" )
  SignalNames.insert( "GSN_INDXATTRINFO", 		"INDXATTRINFO" )
  SignalNames.insert( "GSN_BUILDINDXREQ", 		"BUILDINDXREQ" )
  SignalNames.insert( "GSN_BUILDINDXCONF", 		"BUILDINDXCONF" )
  SignalNames.insert( "GSN_BUILDINDXREF", 		"BUILDINDXREF" )
  //SignalNames.insert( "GSN_TCINDXNEXTREQ", 	"TCINDXNEXTREQ" )
  //SignalNames.insert( "GSN_TCINDEXNEXTCONF", 	"TCINDEXNEXTCONF" )
  //SignalNames.insert( "GSN_TCINDEXNEXREF", 	"TCINDEXNEXREF" )

  SignalNames.insert( "GSN_CREATE_EVNT_REQ",        "CREATE_EVNT_REQ" )
  SignalNames.insert( "GSN_CREATE_EVNT_CONF",       "CREATE_EVNT_CONF" )
  SignalNames.insert( "GSN_CREATE_EVNT_REF",        "CREATE_EVNT_REF" )

  SignalNames.insert( "GSN_SUMA_START_ME_REQ",      "SUMA_START_ME_REQ" )  
  SignalNames.insert( "GSN_SUMA_START_ME_REF",      "SUMA_START_ME_REF" )  
  SignalNames.insert( "GSN_SUMA_START_ME_CONF",     "SUMA_START_ME_CONF" )  
  SignalNames.insert( "GSN_SUMA_HANDOVER_REQ",      "SUMA_HANDOVER_REQ")
  SignalNames.insert( "GSN_SUMA_HANDOVER_REF",      "SUMA_HANDOVER_REF")
  SignalNames.insert( "GSN_SUMA_HANDOVER_CONF",     "SUMA_HANDOVER_CONF") 
  
  SignalNames.insert( "GSN_DROP_EVNT_REQ",          "DROP_EVNT_REQ" )
  SignalNames.insert( "GSN_DROP_EVNT_CONF",         "DROP_EVNT_CONF" )
  SignalNames.insert( "GSN_DROP_EVNT_REF",          "DROP_EVNT_REF" )

  SignalNames.insert( "GSN_BACKUP_TRIG_REQ",        "BACKUP_TRIG_REQ" )
  SignalNames.insert( "GSN_BACKUP_REQ",             "BACKUP_REQ" )
  SignalNames.insert( "GSN_BACKUP_DATA",            "BACKUP_DATA" )
  SignalNames.insert( "GSN_BACKUP_REF",             "BACKUP_REF" )
  SignalNames.insert( "GSN_BACKUP_CONF",            "BACKUP_CONF" )
  SignalNames.insert( "GSN_ABORT_BACKUP_ORD",       "ABORT_BACKUP_ORD" )
  SignalNames.insert( "GSN_BACKUP_ABORT_REP",       "BACKUP_ABORT_REP" )
  SignalNames.insert( "GSN_BACKUP_COMPLETE_REP",    "BACKUP_COMPLETE_REP" )
  SignalNames.insert( "GSN_BACKUP_NF_COMPLETE_REP", "BACKUP_NF_COMPLETE_REP" )
  SignalNames.insert( "GSN_DEFINE_BACKUP_REQ",      "DEFINE_BACKUP_REQ" )
  SignalNames.insert( "GSN_DEFINE_BACKUP_REF",      "DEFINE_BACKUP_REF" )
  SignalNames.insert( "GSN_DEFINE_BACKUP_CONF",     "DEFINE_BACKUP_CONF" )
  SignalNames.insert( "GSN_START_BACKUP_REQ",       "START_BACKUP_REQ" )
  SignalNames.insert( "GSN_START_BACKUP_REF",       "START_BACKUP_REF" )
  SignalNames.insert( "GSN_START_BACKUP_CONF",      "START_BACKUP_CONF" )
  SignalNames.insert( "GSN_BACKUP_FRAGMENT_REQ",    "BACKUP_FRAGMENT_REQ" )
  SignalNames.insert( "GSN_BACKUP_FRAGMENT_REF",    "BACKUP_FRAGMENT_REF" )
  SignalNames.insert( "GSN_BACKUP_FRAGMENT_CONF",   "BACKUP_FRAGMENT_CONF" )
  SignalNames.insert( "GSN_BACKUP_FRAGMENT_COMPLETE_REP",
      "BACKUP_FRAGMENT_COMPLETE_REP" )
  SignalNames.insert( "GSN_STOP_BACKUP_REQ",        "STOP_BACKUP_REQ" )
  SignalNames.insert( "GSN_STOP_BACKUP_REF",        "STOP_BACKUP_REF" )
  SignalNames.insert( "GSN_STOP_BACKUP_CONF",       "STOP_BACKUP_CONF" )
  SignalNames.insert( "GSN_BACKUP_STATUS_REQ",      "BACKUP_STATUS_REQ" )
  SignalNames.insert( "GSN_BACKUP_STATUS_REF",      "BACKUP_STATUS_REF" )
  SignalNames.insert( "GSN_BACKUP_STATUS_CONF",     "BACKUP_STATUS_CONF" )
  SignalNames.insert( "GSN_SIGNAL_DROPPED_REP",     "SIGNAL_DROPPED_REP" )
  SignalNames.insert( "GSN_CONTINUE_FRAGMENTED",    "CONTINUE_FRAGMENTED" )
  SignalNames.insert( "GSN_STOP_FOR_CRASH",         "STOP_FOR_CRASH" )
  SignalNames.insert( "GSN_BACKUP_LOCK_TAB_REQ",    "BACKUP_LOCK_TAB_REQ" )
  SignalNames.insert( "GSN_BACKUP_LOCK_TAB_CONF",   "BACKUP_LOCK_TAB_CONF" )
  SignalNames.insert( "GSN_BACKUP_LOCK_TAB_REF",    "BACKUP_LOCK_TAB_REF" )

  /** Util Block Services **/
  SignalNames.insert( "GSN_UTIL_SEQUENCE_REQ",      "UTIL_SEQUENCE_REQ" )
  SignalNames.insert( "GSN_UTIL_SEQUENCE_REF",      "UTIL_SEQUENCE_REF" )
  SignalNames.insert( "GSN_UTIL_SEQUENCE_CONF",     "UTIL_SEQUENCE_CONF" )
  SignalNames.insert( "GSN_UTIL_PREPARE_REQ",       "UTIL_PREPARE_REQ" )
  SignalNames.insert( "GSN_UTIL_PREPARE_CONF",      "UTIL_PREPARE_CONF" )
  SignalNames.insert( "GSN_UTIL_PREPARE_REF",       "UTIL_PREPARE_REF" )
  SignalNames.insert( "GSN_UTIL_EXECUTE_REQ",       "UTIL_EXECUTE_REQ" )
  SignalNames.insert( "GSN_UTIL_EXECUTE_CONF",      "UTIL_EXECUTE_CONF" )
  SignalNames.insert( "GSN_UTIL_EXECUTE_REF",       "UTIL_EXECUTE_REF" )
  SignalNames.insert( "GSN_UTIL_RELEASE_REQ",       "UTIL_RELEASE_REQ" )
  SignalNames.insert( "GSN_UTIL_RELEASE_CONF",      "UTIL_RELEASE_CONF" )
  SignalNames.insert( "GSN_UTIL_RELEASE_REF",       "UTIL_RELASE_REF" )

  /* Suma Block Services **/
  SignalNames.insert( "GSN_SUB_CREATE_REQ",         "SUB_CREATE_REQ" )
  SignalNames.insert( "GSN_SUB_CREATE_REF",         "SUB_CREATE_REF" )
  SignalNames.insert( "GSN_SUB_CREATE_CONF",        "SUB_CREATE_CONF" )
  SignalNames.insert( "GSN_SUB_REMOVE_REQ",         "SUB_REMOVE_REQ" )
  SignalNames.insert( "GSN_SUB_REMOVE_REF",         "SUB_REMOVE_REF" )
  SignalNames.insert( "GSN_SUB_REMOVE_CONF",        "SUB_REMOVE_CONF" )
  SignalNames.insert( "GSN_SUB_START_REQ",          "SUB_START_REQ" )
  SignalNames.insert( "GSN_SUB_START_REF",          "SUB_START_REF" )
  SignalNames.insert( "GSN_SUB_START_CONF",         "SUB_START_CONF" )
  SignalNames.insert( "GSN_SUB_STOP_REQ",           "SUB_STOP_REQ" )
  SignalNames.insert( "GSN_SUB_STOP_REF",           "SUB_STOP_REF" )
  SignalNames.insert( "GSN_SUB_STOP_CONF",          "SUB_STOP_CONF" )
  SignalNames.insert( "GSN_SUB_SYNC_REQ",           "SUB_SYNC_REQ" )
  SignalNames.insert( "GSN_SUB_SYNC_REF",           "SUB_SYNC_REF" )
  SignalNames.insert( "GSN_SUB_SYNC_CONF",          "SUB_SYNC_CONF" )
  SignalNames.insert( "GSN_SUB_TABLE_DATA",         "SUB_TABLE_DATA" )
  SignalNames.insert( "GSN_SUB_SYNC_CONTINUE_REQ",  "SUB_SYNC_CONTINUE_REQ" )
  SignalNames.insert( "GSN_SUB_SYNC_CONTINUE_REF",  "SUB_SYNC_CONTINUE_REF" )
  SignalNames.insert( "GSN_SUB_SYNC_CONTINUE_CONF", "SUB_SYNC_CONTINUE_CONF" )
  SignalNames.insert( "GSN_SUB_GCP_COMPLETE_REP",   "SUB_GCP_COMPLETE_REP" )
  SignalNames.insert( "GSN_SUB_GCP_COMPLETE_ACK",   "SUB_GCP_COMPLETE_ACK" )

  SignalNames.insert( "GSN_CREATE_SUBID_REQ",         "CREATE_SUBID_REQ" )
  SignalNames.insert( "GSN_CREATE_SUBID_REF",         "CREATE_SUBID_REF" )
  SignalNames.insert( "GSN_CREATE_SUBID_CONF",        "CREATE_SUBID_CONF" )

  SignalNames.insert( "GSN_CREATE_TABLE_REQ",       "CREATE_TABLE_REQ" )
  SignalNames.insert( "GSN_CREATE_TABLE_REF",       "CREATE_TABLE_REF" )
  SignalNames.insert( "GSN_CREATE_TABLE_CONF",      "CREATE_TABLE_CONF" )

  SignalNames.insert( "GSN_CREATE_TAB_REQ",         "CREATE_TAB_REQ" )
  SignalNames.insert( "GSN_CREATE_TAB_REF",         "CREATE_TAB_REF" )
  SignalNames.insert( "GSN_CREATE_TAB_CONF",        "CREATE_TAB_CONF" )
  
  SignalNames.insert( "GSN_ALTER_TABLE_REQ",          "ALTER_TABLE_REQ" )
  SignalNames.insert( "GSN_ALTER_TABLE_REF",          "ALTER_TABLE_REF" )
  SignalNames.insert( "GSN_ALTER_TABLE_CONF",         "ALTER_TABLE_CONF" )
  
  SignalNames.insert( "GSN_ALTER_TAB_REQ",          "ALTER_TAB_REQ" )
  SignalNames.insert( "GSN_ALTER_TAB_REF",          "ALTER_TAB_REF" )
  SignalNames.insert( "GSN_ALTER_TAB_CONF",         "ALTER_TAB_CONF" )
  
  SignalNames.insert( "GSN_CREATE_FRAGMENTATION_REQ",  "CREATE_FRAGMENTATION_REQ" )
  SignalNames.insert( "GSN_CREATE_FRAGMENTATION_REF",  "CREATE_FRAGMENTATION_REF" )
  SignalNames.insert( "GSN_CREATE_FRAGMENTATION_CONF", "CREATE_FRAGMENTATION_CONF" )

  SignalNames.insert( "GSN_SET_WAKEUP_THREAD_ORD",  "SET_WAKEUP_THREAD_ORD" )
  SignalNames.insert( "GSN_WAKEUP_THREAD_ORD",      "WAKEUP_THREAD_ORD" )
  SignalNames.insert( "GSN_SEND_WAKEUP_THREAD_ORD",  "SEND_WAKEUP_THREAD_ORD" )

  SignalNames.insert( "GSN_UTIL_CREATE_LOCK_REQ",   "UTIL_CREATE_LOCK_REQ" )
  SignalNames.insert( "GSN_UTIL_CREATE_LOCK_REF",   "UTIL_CREATE_LOCK_REF" )
  SignalNames.insert( "GSN_UTIL_CREATE_LOCK_CONF",  "UTIL_CREATE_LOCK_CONF" )
  SignalNames.insert( "GSN_UTIL_DESTROY_LOCK_REQ",  "UTIL_DESTROY_LOCK_REQ" )
  SignalNames.insert( "GSN_UTIL_DESTROY_LOCK_REF",  "UTIL_DESTROY_LOCK_REF" )
  SignalNames.insert( "GSN_UTIL_DESTROY_LOCK_CONF", "UTIL_DESTROY_LOCK_CONF" )
  SignalNames.insert( "GSN_UTIL_LOCK_REQ",          "UTIL_LOCK_REQ" )
  SignalNames.insert( "GSN_UTIL_LOCK_REF",          "UTIL_LOCK_REF" )
  SignalNames.insert( "GSN_UTIL_LOCK_CONF",         "UTIL_LOCK_CONF" )
  SignalNames.insert( "GSN_UTIL_UNLOCK_REQ",        "UTIL_UNLOCK_REQ" )
  SignalNames.insert( "GSN_UTIL_UNLOCK_REF",        "UTIL_UNLOCK_REF" )
  SignalNames.insert( "GSN_UTIL_UNLOCK_CONF",       "UTIL_UNLOCK_CONF" )

  /* TUX */
  SignalNames.insert( "GSN_TUXFRAGREQ",  "TUXFRAGREQ" )
  SignalNames.insert( "GSN_TUXFRAGCONF", "TUXFRAGCONF" )
  SignalNames.insert( "GSN_TUXFRAGREF",  "TUXFRAGREF" )
  SignalNames.insert( "GSN_TUX_ADD_ATTRREQ",  "TUX_ADD_ATTRREQ" )
  SignalNames.insert( "GSN_TUX_ADD_ATTRCONF", "TUX_ADD_ATTRCONF" )
  SignalNames.insert( "GSN_TUX_ADD_ATTRREF",  "TUX_ADD_ATTRREF" )
  SignalNames.insert( "GSN_TUX_MAINT_REQ",  "TUX_MAINT_REQ" )
  SignalNames.insert( "GSN_TUX_MAINT_CONF", "TUX_MAINT_CONF" )
  SignalNames.insert( "GSN_TUX_MAINT_REF",  "TUX_MAINT_REF" )
  SignalNames.insert( "GSN_TUX_BOUND_INFO",  "TUX_BOUND_INFO" )
  SignalNames.insert( "GSN_ACC_LOCKREQ",  "ACC_LOCKREQ" )

  SignalNames.insert( "GSN_CREATE_FILEGROUP_REQ", "CREATE_FILEGROUP_REQ" )
  SignalNames.insert( "GSN_CREATE_FILEGROUP_REF", "CREATE_FILEGROUP_REF" )
  SignalNames.insert( "GSN_CREATE_FILEGROUP_CONF", "CREATE_FILEGROUP_CONF" )
  
  SignalNames.insert( "GSN_CREATE_FILE_REQ",  "CREATE_FILE_REQ" )
  SignalNames.insert( "GSN_CREATE_FILE_REF",  "CREATE_FILE_REF" )
  SignalNames.insert( "GSN_CREATE_FILE_CONF", "CREATE_FILE_CONF" )
  
  SignalNames.insert( "GSN_DROP_FILEGROUP_REQ",  "DROP_FILEGROUP_REQ" )
  SignalNames.insert( "GSN_DROP_FILEGROUP_REF",  "DROP_FILEGROUP_REF" )
  SignalNames.insert( "GSN_DROP_FILEGROUP_CONF", "DROP_FILEGROUP_CONF" )
  
  SignalNames.insert( "GSN_DROP_FILE_REQ",  "DROP_FILE_REQ" )
  SignalNames.insert( "GSN_DROP_FILE_REF",  "DROP_FILE_REF" )
  SignalNames.insert( "GSN_DROP_FILE_CONF", "DROP_FILE_CONF" )
  
  SignalNames.insert( "GSN_CREATE_FILEGROUP_IMPL_REQ", "CREATE_FILEGROUP_IMPL_REQ" )
  SignalNames.insert( "GSN_CREATE_FILEGROUP_IMPL_REF", "CREATE_FILEGROUP_IMPL_REF" )
  SignalNames.insert( "GSN_CREATE_FILEGROUP_IMPL_CONF", "CREATE_FILEGROUP_IMPL_CONF" )
  
  SignalNames.insert( "GSN_CREATE_FILE_IMPL_REQ",  "CREATE_FILE_IMPL_REQ" )
  SignalNames.insert( "GSN_CREATE_FILE_IMPL_REF",  "CREATE_FILE_IMPL_REF" )
  SignalNames.insert( "GSN_CREATE_FILE_IMPL_CONF", "CREATE_FILE_IMPL_CONF" )

  SignalNames.insert( "GSN_DROP_FILEGROUP_IMPL_REQ",  "DROP_FILEGROUP_IMPL_REQ" )
  SignalNames.insert( "GSN_DROP_FILEGROUP_IMPL_REF",  "DROP_FILEGROUP_IMPL_REF" )
  SignalNames.insert( "GSN_DROP_FILEGROUP_IMPL_CONF", "DROP_FILEGROUP_IMPL_CONF" )

  SignalNames.insert( "GSN_DROP_FILE_IMPL_REQ",  "DROP_FILE_IMPL_REQ" )
  SignalNames.insert( "GSN_DROP_FILE_IMPL_REF",  "DROP_FILE_IMPL_REF" )
  SignalNames.insert( "GSN_DROP_FILE_IMPL_CONF", "DROP_FILE_IMPL_CONF" )
  
  SignalNames.insert( "GSN_LCP_PREPARE_REQ",  "LCP_PREPARE_REQ" )
  SignalNames.insert( "GSN_LCP_PREPARE_REF",  "LCP_PREPARE_REF" )
  SignalNames.insert( "GSN_LCP_PREPARE_CONF", "LCP_PREPARE_CONF" )

  SignalNames.insert( "GSN_CHECK_NODE_RESTARTREQ",  "CHECK_NODE_RESTARTREQ" )
  SignalNames.insert( "GSN_CHECK_NODE_RESTARTCONF", "CHECK_NODE_RESTARTCONF" )

  SignalNames.insert( "GSN_GET_CPU_USAGE_REQ", "GET_CPU_USAGE_REQ" )

  SignalNames.insert( "GSN_OVERLOAD_STATUS_REP", "OVERLOAD_STATUS_REP" )
  SignalNames.insert( "GSN_SEND_THREAD_STATUS_REP", "SEND_THREAD_STATUS_REP" )
  SignalNames.insert( "GSN_NODE_OVERLOAD_STATUS_ORD", "NODE_OVERLOAD_STATUS_ORD" )

  /* DICT LOCK */
  SignalNames.insert( "GSN_DICT_LOCK_REQ",          "DICT_LOCK_REQ" )
  SignalNames.insert( "GSN_DICT_LOCK_CONF",         "DICT_LOCK_CONF" )
  SignalNames.insert( "GSN_DICT_LOCK_REF",          "DICT_LOCK_REF" )
  SignalNames.insert( "GSN_DICT_UNLOCK_ORD",        "DICT_UNLOCK_ORD" )

  SignalNames.insert( "GSN_DICT_TAKEOVER_REQ",  "DICT_TAKEOVER_REQ" )
  SignalNames.insert( "GSN_DICT_TAKEOVER_REF",  "DICT_TAKEOVER_REF" )
  SignalNames.insert( "GSN_DICT_TAKEOVER_CONF", "DICT_TAKEOVER_CONF" )

  SignalNames.insert( "GSN_UPDATE_FRAG_DIST_KEY_ORD", "UPDATE_FRAG_DIST_KEY_ORD" )

  SignalNames.insert( "GSN_ROUTE_ORD", "ROUTE_ORD" )
  SignalNames.insert( "GSN_NODE_VERSION_REP", "NODE_VERSION_REP" )

  SignalNames.insert( "GSN_PREPARE_COPY_FRAG_REQ",   "PREPARE_COPY_FRAG_REQ" )
  SignalNames.insert( "GSN_PREPARE_COPY_FRAG_REF",   "PREPARE_COPY_FRAG_REF" )
  SignalNames.insert( "GSN_PREPARE_COPY_FRAG_CONF",  "PREPARE_COPY_FRAG_CONF" )

  SignalNames.insert( "GSN_UPGRADE_PROTOCOL_ORD", "UPGRADE_PROTOCOL_ORD" )

  SignalNames.insert( "GSN_TC_HBREP", "TC_HBREP" )
  
  SignalNames.insert( "GSN_START_TOREF", "START_TOREF" )
  SignalNames.insert( "GSN_END_TOREF", "END_TOREF" )
  SignalNames.insert( "GSN_START_PERMREP", "START_PERMREP" )

  SignalNames.insert( "GSN_SCHEMA_TRANS_BEGIN_REQ", "SCHEMA_TRANS_BEGIN_REQ" )
  SignalNames.insert( "GSN_SCHEMA_TRANS_BEGIN_CONF", "SCHEMA_TRANS_BEGIN_CONF" )
  SignalNames.insert( "GSN_SCHEMA_TRANS_BEGIN_REF", "SCHEMA_TRANS_BEGIN_REF" )
  SignalNames.insert( "GSN_SCHEMA_TRANS_END_REQ", "SCHEMA_TRANS_END_REQ" )
  SignalNames.insert( "GSN_SCHEMA_TRANS_END_CONF", "SCHEMA_TRANS_END_CONF" )
  SignalNames.insert( "GSN_SCHEMA_TRANS_END_REF", "SCHEMA_TRANS_END_REF" )
  SignalNames.insert( "GSN_SCHEMA_TRANS_END_REP", "SCHEMA_TRANS_END_REP" )
  SignalNames.insert( "GSN_SCHEMA_TRANS_IMPL_REQ", "SCHEMA_TRANS_IMPL_REQ" )
  SignalNames.insert( "GSN_SCHEMA_TRANS_IMPL_CONF", "SCHEMA_TRANS_IMPL_CONF" )
  SignalNames.insert( "GSN_SCHEMA_TRANS_IMPL_REF", "SCHEMA_TRANS_IMPL_REF" )

  SignalNames.insert( "GSN_CREATE_TRIG_IMPL_REQ", "CREATE_TRIG_IMPL_REQ" )
  SignalNames.insert( "GSN_CREATE_TRIG_IMPL_CONF", "CREATE_TRIG_IMPL_CONF" )
  SignalNames.insert( "GSN_CREATE_TRIG_IMPL_REF", "CREATE_TRIG_IMPL_REF" )
  SignalNames.insert( "GSN_DROP_TRIG_IMPL_REQ", "DROP_TRIG_IMPL_REQ" )
  SignalNames.insert( "GSN_DROP_TRIG_IMPL_CONF", "DROP_TRIG_IMPL_CONF" )
  SignalNames.insert( "GSN_DROP_TRIG_IMPL_REF", "DROP_TRIG_IMPL_REF" )
  SignalNames.insert( "GSN_ALTER_TRIG_IMPL_REQ", "ALTER_TRIG_IMPL_REQ" )
  SignalNames.insert( "GSN_ALTER_TRIG_IMPL_CONF", "ALTER_TRIG_IMPL_CONF" )
  SignalNames.insert( "GSN_ALTER_TRIG_IMPL_REF", "ALTER_TRIG_IMPL_REF" )

  SignalNames.insert( "GSN_CREATE_INDX_IMPL_REQ", "CREATE_INDX_IMPL_REQ" )
  SignalNames.insert( "GSN_CREATE_INDX_IMPL_CONF", "CREATE_INDX_IMPL_CONF" )
  SignalNames.insert( "GSN_CREATE_INDX_IMPL_REF", "CREATE_INDX_IMPL_REF" )
  SignalNames.insert( "GSN_DROP_INDX_IMPL_REQ", "DROP_INDX_IMPL_REQ" )
  SignalNames.insert( "GSN_DROP_INDX_IMPL_CONF", "DROP_INDX_IMPL_CONF" )
  SignalNames.insert( "GSN_DROP_INDX_IMPL_REF", "DROP_INDX_IMPL_REF" )
  SignalNames.insert( "GSN_ALTER_INDX_IMPL_REQ", "ALTER_INDX_IMPL_REQ" )
  SignalNames.insert( "GSN_ALTER_INDX_IMPL_CONF", "ALTER_INDX_IMPL_CONF" )
  SignalNames.insert( "GSN_ALTER_INDX_IMPL_REF", "ALTER_INDX_IMPL_REF" )

  SignalNames.insert( "GSN_DROP_FRAG_REQ",  "DROP_FRAG_REQ" )
  SignalNames.insert( "GSN_DROP_FRAG_REF",  "DROP_FRAG_REF" )
  SignalNames.insert( "GSN_DROP_FRAG_CONF", "DROP_FRAG_CONF" )

  SignalNames.insert( "GSN_BUILD_INDX_IMPL_REQ", "BUILD_INDX_IMPL_REQ" )
  SignalNames.insert( "GSN_BUILD_INDX_IMPL_CONF", "BUILD_INDX_IMPL_CONF" )
  SignalNames.insert( "GSN_BUILD_INDX_IMPL_REF", "BUILD_INDX_IMPL_REF" )

  SignalNames.insert( "GSN_RESTORE_LCP_REQ", "RESTORE_LCP_REQ" )
  SignalNames.insert( "GSN_RESTORE_LCP_CONF", "RESTORE_LCP_CONF" )
  SignalNames.insert( "GSN_RESTORE_LCP_REF", "RESTORE_LCP_REF" )

  SignalNames.insert( "GSN_CREATE_NODEGROUP_REQ", "CREATE_NODEGROUP_REQ" )
  SignalNames.insert( "GSN_CREATE_NODEGROUP_CONF", "CREATE_NODEGROUP_CONF" )
  SignalNames.insert( "GSN_CREATE_NODEGROUP_REF", "CREATE_NODEGROUP_REF" )

  SignalNames.insert( "GSN_CREATE_NODEGROUP_IMPL_REQ", "CREATE_NODEGROUP_IMPL_REQ" )
  SignalNames.insert( "GSN_CREATE_NODEGROUP_IMPL_CONF", "CREATE_NODEGROUP_IMPL_CONF" )
  SignalNames.insert( "GSN_CREATE_NODEGROUP_IMPL_REF", "CREATE_NODEGROUP_IMPL_REF" )

  SignalNames.insert( "GSN_DROP_NODEGROUP_REQ", "DROP_NODEGROUP_REQ" )
  SignalNames.insert( "GSN_DROP_NODEGROUP_CONF", "DROP_NODEGROUP_CONF" )
  SignalNames.insert( "GSN_DROP_NODEGROUP_REF", "DROP_NODEGROUP_REF" )

  SignalNames.insert( "GSN_DROP_NODEGROUP_IMPL_REQ", "DROP_NODEGROUP_IMPL_REQ" )
  SignalNames.insert( "GSN_DROP_NODEGROUP_IMPL_CONF", "DROP_NODEGROUP_IMPL_CONF" )
  SignalNames.insert( "GSN_DROP_NODEGROUP_IMPL_REF", "DROP_NODEGROUP_IMPL_REF" )

  SignalNames.insert( "GSN_CONFIG_CHECK_REQ", "CONFIG_CHECK_REQ" )
  SignalNames.insert( "GSN_CONFIG_CHECK_REF", "CONFIG_CHECK_REF" )
  SignalNames.insert( "GSN_CONFIG_CHECK_CONF", "CONFIG_CHECK_CONF" )

  SignalNames.insert( "GSN_CONFIG_CHANGE_REQ", "CONFIG_CHANGE_REQ" )
  SignalNames.insert( "GSN_CONFIG_CHANGE_REF", "CONFIG_CHANGE_REF" )
  SignalNames.insert( "GSN_CONFIG_CHANGE_CONF", "CONFIG_CHANGE_CONF" )

  SignalNames.insert( "GSN_CONFIG_CHANGE_IMPL_REQ", "CONFIG_CHANGE_IMPL_REQ" )
  SignalNames.insert( "GSN_CONFIG_CHANGE_IMPL_REF", "CONFIG_CHANGE_IMPL_REF" )
  SignalNames.insert( "GSN_CONFIG_CHANGE_IMPL_CONF", "CONFIG_CHANGE_IMPL_CONF" )

  SignalNames.insert( "GSN_DATA_FILE_ORD", "DATA_FILE_ORD" )

  SignalNames.insert( "GSN_CALLBACK_REQ", "CALLBACK_REQ" )
  SignalNames.insert( "GSN_CALLBACK_CONF", "CALLBACK_CONF" )
  SignalNames.insert( "GSN_CALLBACK_ACK", "CALLBACK_ACK" )

  SignalNames.insert( "GSN_RELEASE_PAGES_REQ", "RELEASE_PAGES_REQ" )
  SignalNames.insert( "GSN_RELEASE_PAGES_CONF", "RELEASE_PAGES_CONF" )

  SignalNames.insert( "GSN_CREATE_HASH_MAP_REQ", "CREATE_HASH_MAP_REQ" )
  SignalNames.insert( "GSN_CREATE_HASH_MAP_REF", "CREATE_HASH_MAP_REF" )
  SignalNames.insert( "GSN_CREATE_HASH_MAP_CONF", "CREATE_HASH_MAP_CONF" )

  SignalNames.insert( "GSN_SYNC_THREAD_REQ", "SYNC_THREAD_REQ" )
  SignalNames.insert( "GSN_SYNC_THREAD_CONF", "SYNC_THREAD_CONF" )

  SignalNames.insert( "GSN_LOCAL_ROUTE_ORD", "LOCAL_ROUTE_ORD" )

  SignalNames.insert( "GSN_ALLOC_MEM_REQ", "ALLOC_MEM_REQ" )
  SignalNames.insert( "GSN_ALLOC_MEM_REF", "ALLOC_MEM_REF" )
  SignalNames.insert( "GSN_ALLOC_MEM_CONF", "ALLOC_MEM_CONF" )

  SignalNames.insert( "GSN_DIH_GET_TABINFO_REQ", "DIH_GET_TABINFO_REQ" )
  SignalNames.insert( "GSN_DIH_GET_TABINFO_REF", "DIH_GET_TABINFO_REF" )
  SignalNames.insert( "GSN_DIH_GET_TABINFO_CONF", "DIH_GET_TABINFO_CONF" )

  SignalNames.insert( "GSN_SYNC_REQ", "SYNC_REQ" )
  SignalNames.insert( "GSN_SYNC_REF", "SYNC_REF" )
  SignalNames.insert( "GSN_SYNC_CONF", "SYNC_CONF" )

  SignalNames.insert( "GSN_SYNC_PATH_REQ", "SYNC_PATH_REQ" )
  SignalNames.insert( "GSN_SYNC_PATH_CONF", "SYNC_PATH_CONF" )

  SignalNames.insert( "GSN_NODE_PING_REQ", "NODE_PING_REQ" )
  SignalNames.insert( "GSN_NODE_PING_CONF", "NODE_PING_CONF" )

  SignalNames.insert( "GSN_INDEX_STAT_REQ", "INDEX_STAT_REQ" )
  SignalNames.insert( "GSN_INDEX_STAT_CONF", "INDEX_STAT_CONF" )
  SignalNames.insert( "GSN_INDEX_STAT_REF", "INDEX_STAT_REF" )
  SignalNames.insert( "GSN_INDEX_STAT_IMPL_REQ", "INDEX_STAT_IMPL_REQ" )
  SignalNames.insert( "GSN_INDEX_STAT_IMPL_CONF", "INDEX_STAT_IMPL_CONF" )
  SignalNames.insert( "GSN_INDEX_STAT_IMPL_REF", "INDEX_STAT_IMPL_REF" )
  SignalNames.insert( "GSN_INDEX_STAT_REP", "INDEX_STAT_REP" )

  SignalNames.insert( "GSN_GET_CONFIG_REQ", "GET_CONFIG_REQ" )
  SignalNames.insert( "GSN_GET_CONFIG_REF", "GET_CONFIG_REF" )
  SignalNames.insert( "GSN_GET_CONFIG_CONF", "GET_CONFIG_CONF" )

  SignalNames.insert( "GSN_ALLOC_NODEID_REQ", "ALLOC_NODEID_REQ" )
  SignalNames.insert( "GSN_ALLOC_NODEID_CONF", "ALLOC_NODEID_CONF" )
  SignalNames.insert( "GSN_ALLOC_NODEID_REF", "ALLOC_NODEID_REF" )

  SignalNames.insert( "GSN_LCP_STATUS_REQ", "LCP_STATUS_REQ" )
  SignalNames.insert( "GSN_LCP_STATUS_CONF", "LCP_STATUS_CONF" )
  SignalNames.insert( "GSN_LCP_STATUS_REF", "LCP_STATUS_REF" )

  SignalNames.insert( "GSN_CREATE_FK_REQ", "CREATE_FK_REQ" )
  SignalNames.insert( "GSN_CREATE_FK_REF", "CREATE_FK_REF" )
  SignalNames.insert( "GSN_CREATE_FK_CONF", "CREATE_FK_CONF" )
  SignalNames.insert( "GSN_DROP_FK_REQ", "DROP_FK_REQ" )
  SignalNames.insert( "GSN_DROP_FK_REF", "DROP_FK_REF" )
  SignalNames.insert( "GSN_DROP_FK_CONF", "DROP_FK_CONF" )
  SignalNames.insert( "GSN_CREATE_FK_IMPL_REQ", "CREATE_FK_IMPL_REQ" )
  SignalNames.insert( "GSN_CREATE_FK_IMPL_REF", "CREATE_FK_IMPL_REF" )
  SignalNames.insert( "GSN_CREATE_FK_IMPL_CONF", "CREATE_FK_IMPL_CONF" )
  SignalNames.insert( "GSN_DROP_FK_IMPL_REQ", "DROP_FK_IMPL_REQ" )
  SignalNames.insert( "GSN_DROP_FK_IMPL_REF", "DROP_FK_IMPL_REF" )
  SignalNames.insert( "GSN_DROP_FK_IMPL_CONF", "DROP_FK_IMPL_CONF" )
  SignalNames.insert( "GSN_BUILD_FK_REQ", "BUILD_FK_REQ" )
  SignalNames.insert( "GSN_BUILD_FK_REF", "BUILD_FK_REF" )
  SignalNames.insert( "GSN_BUILD_FK_CONF", "BUILD_FK_CONF" )
  SignalNames.insert( "GSN_BUILD_FK_IMPL_REQ", "BUILD_FK_IMPL_REQ" )
  SignalNames.insert( "GSN_BUILD_FK_IMPL_REF", "BUILD_FK_IMPL_REF" )
  SignalNames.insert( "GSN_BUILD_FK_IMPL_CONF", "BUILD_FK_IMPL_CONF" )
  SignalNames.insert( "GSN_NODE_STARTED_REP", "NODE_STARTED_REP" )
  SignalNames.insert( "GSN_PAUSE_LCP_REQ", "PAUSE_LCP_REQ" )
  SignalNames.insert( "GSN_PAUSE_LCP_CONF", "PAUSE_LCP_CONF" )
  SignalNames.insert( "GSN_FLUSH_LCP_REP_REQ", "FLUSH_LCP_REP_REQ" )
  SignalNames.insert( "GSN_FLUSH_LCP_REP_CONF", "FLUSH_LCP_REP_CONF" )
  SignalNames.insert( "GSN_ALLOC_NODEID_REP", "ALLOC_NODEID_REP" )
  SignalNames.insert( "GSN_INCL_NODE_HB_PROTOCOL_REP", "INCL_NODE_HB_PROTOCOL_REP" )
  SignalNames.insert( "GSN_NDBCNTR_START_WAIT_REP", "NDBCNTR_START_WAIT_REP" )
  SignalNames.insert( "GSN_NDBCNTR_STARTED_REP", "NDBCNTR_STARTED_REP" )
  SignalNames.insert( "GSN_SUMA_HANDOVER_COMPLETE_REP", "SUMA_HANDOVER_COMPLETE_REP" )
  SignalNames.insert( "GSN_END_TOREP", "END_TOREP" )
  SignalNames.insert( "GSN_LOCAL_RECOVERY_COMP_REP", "LOCAL_RECOVERY_COMP_REP" )
  SignalNames.insert( "GSN_CANCEL_SUBSCRIPTION_REQ", "CANCEL_SUBSCRIPTION_REQ" )
  SignalNames.insert( "GSN_ISOLATE_ORD", "ISOLATE_ORD" )
  SignalNames.insert( "GSN_PROCESSINFO_REP", "PROCESSINFO_REP" )
  SignalNames.insert( "GSN_SYNC_PAGE_CACHE_REQ", "SYNC_PAGE_CACHE_REQ" )
  SignalNames.insert( "GSN_SYNC_PAGE_CACHE_CONF", "SYNC_PAGE_CACHE_CONF" )
  SignalNames.insert( "GSN_SYNC_EXTENT_PAGES_REQ", "SYNC_EXTENT_PAGES_REQ" )
  SignalNames.insert( "GSN_SYNC_EXTENT_PAGES_CONF", "SYNC_EXTENT_PAGES_CONF" )
  SignalNames.insert( "GSN_RESTORABLE_GCI_REP", "RESTORABLE_GCI_REP" )
  SignalNames.insert( "GSN_WAIT_ALL_COMPLETE_LCP_REQ", "WAIT_ALL_COMPLETE_LCP_REQ" )
  SignalNames.insert( "GSN_WAIT_ALL_COMPLETE_LCP_CONF", "WAIT_ALL_COMPLETE_LCP_CONF" )
  SignalNames.insert( "GSN_WAIT_COMPLETE_LCP_REQ", "WAIT_COMPLETE_LCP_REQ" )
  SignalNames.insert( "GSN_WAIT_COMPLETE_LCP_CONF", "WAIT_COMPLETE_LCP_CONF" )
  SignalNames.insert( "GSN_INFO_GCP_STOP_TIMER", "INFO_STOP_GCP_TIMER" )
  SignalNames.insert( "GSN_READ_LOCAL_SYSFILE_REQ", "READ_LOCAL_SYSFILE_REQ" )
  SignalNames.insert( "GSN_READ_LOCAL_SYSFILE_CONF", "READ_LOCAL_SYSFILE_CONF" )
  SignalNames.insert( "GSN_WRITE_LOCAL_SYSFILE_REQ", "WRITE_LOCAL_SYSFILE_REQ" )
  SignalNames.insert( "GSN_WRITE_LOCAL_SYSFILE_CONF", "WRITE_LOCAL_SYSFILE_CONF" )
  SignalNames.insert( "GSN_GET_LATEST_GCI_REQ", "GET_LATEST_GCI_REQ" )
  SignalNames.insert( "GSN_HALT_COPY_FRAG_REQ", "HALT_COPY_FRAG_REQ" )
  SignalNames.insert( "GSN_HALT_COPY_FRAG_CONF", "HALT_COPY_FRAG_CONF" )
  SignalNames.insert( "GSN_HALT_COPY_FRAG_REF", "HALT_COPY_FRAG_REF" )
  SignalNames.insert( "GSN_RESUME_COPY_FRAG_REQ", "RESUME_COPY_FRAG_REQ" )
  SignalNames.insert( "GSN_RESUME_COPY_FRAG_CONF", "RESUME_COPY_FRAG_CONF" )
  SignalNames.insert( "GSN_RESUME_COPY_FRAG_REF", "RESUME_COPY_FRAG_REF" )
  SignalNames.insert( "GSN_START_LOCAL_LCP_ORD", "START_LOCAL_LCP_ORD" )
  SignalNames.insert( "GSN_START_FULL_LOCAL_LCP_ORD", "START_FULL_LOCAL_LCP_ORD" )
  SignalNames.insert( "GSN_START_DISTRIBUTED_LCP_ORD", "START_DISTRIBUTED_LCP_ORD" )
  SignalNames.insert( "GSN_CUT_UNDO_LOG_TAIL_REQ", "CUT_UNDO_LOG_TAIL_REQ" )
  SignalNames.insert( "GSN_CUT_UNDO_LOG_TAIL_CONF", "CUT_UNDO_LOG_TAIL_CONF" )
  SignalNames.insert( "GSN_CUT_REDO_LOG_TAIL_REQ", "CUT_REDO_LOG_TAIL_REQ" )
  SignalNames.insert( "GSN_CUT_REDO_LOG_TAIL_CONF", "CUT_REDO_LOG_TAIL_CONF" )
  SignalNames.insert( "GSN_LCP_ALL_COMPLETE_REQ", "LCP_ALL_COMPLETE_REQ" )
  SignalNames.insert( "GSN_LCP_ALL_COMPLETE_CONF", "LCP_ALL_COMPLETE_CONF" )
  SignalNames.insert( "GSN_COPY_FRAG_IN_PROGRESS_REP", "COPY_FRAG_IN_PROGRESS_REP" )
  SignalNames.insert( "GSN_COPY_FRAG_NOT_IN_PROGRESS_REP",
     "COPY_FRAG_NOT_IN_PROGRESS_REP" )
  SignalNames.insert( "GSN_SET_LOCAL_LCP_ID_REQ", "SET_LOCAL_LCP_ID_REQ" )
  SignalNames.insert( "GSN_SET_LOCAL_LCP_ID_CONF", "SET_LOCAL_LCP_ID_CONF" )
  SignalNames.insert( "GSN_START_NODE_LCP_REQ", "START_NODE_LCP_REQ" )
  SignalNames.insert( "GSN_START_NODE_LCP_CONF", "START_NODE_LCP_CONF" )
  SignalNames.insert( "GSN_UNDO_LOG_LEVEL_REP", "UNDO_LOG_LEVEL_REP" )
  SignalNames.insert( "GSN_LCP_START_REP", "LCP_START_REP" )
  SignalNames.insert( "GSN_INFORM_BACKUP_DROP_TAB_REQ", "INFORM_BACKUP_DROP_TAB_REQ" )
  SignalNames.insert( "GSN_INFORM_BACKUP_DROP_TAB_CONF", "INFORM_BACKUP_DROP_TAB_CONF" )
  SignalNames.insert( "GSN_CHECK_LCP_IDLE_ORD", "CHECK_LCP_IDLE_ORD" )
  SignalNames.insert( "GSN_SET_LATEST_LCP_ID", "SET_LATEST_LCP_ID" )
  SignalNames.insert( "GSN_SYNC_PAGE_WAIT_REP", "SYNC_PAGE_WAIT_REP" )
  SignalNames.insert( "GSN_REDO_STATE_REP", "REDO_STATE_REP" )
  SignalNames.insert( "GSN_WAIT_LCP_IDLE_REQ", "WAIT_LCP_IDLE_REQ" )
  SignalNames.insert( "GSN_WAIT_LCP_IDLE_CONF", "WAIT_LCP_IDLE_CONF" )
  SignalNames.insert( "GSN_LOCAL_LATEST_LCP_ID_REP", "LOCAL_LATEST_LCP_ID_REP" )
  SignalNames.insert( "GSN_SYNC_THREAD_VIA_REQ", "SYNC_THREAD_VIA_REQ" )
  SignalNames.insert( "GSN_SYNC_THREAD_VIA_CONF", "SYNC_THREAD_VIA_CONF" )
  SignalNames.insert( "GSN_SET_UP_MULTI_TRP_REQ", "SET_UP_MULTI_TRP_REQ" )
  SignalNames.insert( "GSN_SET_UP_MULTI_TRP_CONF", "SET_UP_MULTI_TRP_CONF" )
  SignalNames.insert( "GSN_GET_NUM_MULTI_TRP_REQ", "GET_NUM_MULTI_TRP_REQ" )
  SignalNames.insert( "GSN_GET_NUM_MULTI_TRP_CONF", "GET_NUM_MULTI_TRP_CONF" )
  SignalNames.insert( "GSN_GET_NUM_MULTI_TRP_REF", "GET_NUM_MULTI_TRP_REF" )
  SignalNames.insert( "GSN_FREEZE_THREAD_REQ", "FREEZE_THREAD_REQ" )
  SignalNames.insert( "GSN_FREEZE_THREAD_CONF", "FREEZE_THREAD_CONF" )
  SignalNames.insert( "GSN_FREEZE_ACTION_REQ", "FREEZE_ACTION_REQ" )
  SignalNames.insert( "GSN_FREEZE_ACTION_CONF", "FREEZE_ACTION_CONF" )
  SignalNames.insert( "GSN_ACTIVATE_TRP_REQ", "ACTIVATE_TRP_REQ" )
  SignalNames.insert( "GSN_ACTIVATE_TRP_CONF", "ACTIVATE_TRP_CONF" )
  SignalNames.insert( "GSN_SWITCH_MULTI_TRP_REQ", "SWITCH_MULTI_TRP_REQ" )
  SignalNames.insert( "GSN_SWITCH_MULTI_TRP_CONF", "SWITCH_MULTI_TRP_CONF" )
  SignalNames.insert( "GSN_SWITCH_MULTI_TRP_REF", "SWITCH_MULTI_TRP_REF" )
  SignalNames.insert( "GSN_MEASURE_WAKEUP_TIME_ORD", "MEASURE_WAKEUP_TIME_ORD" )
  SignalNames.insert( "GSN_UPD_QUERY_DIST_ORD", "UPD_QUERY_DIST_ORD" )
  SignalNames.insert( "GSN_UPD_THR_LOAD_ORD", "UPDATE_THR_LOAD_ORD" )
  SignalNames.insert( "GSN_ACTIVATE_REQ", "ACTIVATE_REQ" );
  SignalNames.insert( "GSN_ACTIVATE_CONF", "ACTIVATE_CONF" );
  SignalNames.insert( "GSN_ACTIVATE_REF", "ACTIVATE_REF" );
  SignalNames.insert( "GSN_DEACTIVATE_REQ", "DEACTIVATE_REQ" );
  SignalNames.insert( "GSN_DEACTIVATE_CONF", "DEACTIVATE_CONF" );
  SignalNames.insert( "GSN_DEACTIVATE_REF", "DEACTIVATE_REF" );
  SignalNames.insert( "GSN_SET_HOSTNAME_REQ", "SET_HOSTNAME_REQ" );
  SignalNames.insert( "GSN_SET_HOSTNAME_CONF", "SET_HOSTNAME_CONF" );
  SignalNames.insert( "GSN_SET_HOSTNAME_REF", "SET_HOSTNAME_REF" );
);
const let unsigned NO_OF_SIGNAL_NAMES: i16 = sizeof(SignalNames)/sizeof(GSNName);