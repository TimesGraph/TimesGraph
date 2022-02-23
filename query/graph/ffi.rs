use crate::parser;
use crate::planner::QueryPlan;
use crate::runtime::{Program, Status, VirtualMachine};
use crate::store::{PropOwned, Store, StoreTxn};
use crate::Error;
use std::collections::HashMap;
use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::ptr::read;
use std::sync::atomic::{AtomicUsize, Ordering};

#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum TimesGraphStatus {
    TG_OK = 0,
    TG_MATCH = 1,
    TG_DONE = 2,

    // Errors
    TG_IO = 100,
    TG_CORRUPTION = 101,
    TG_POISON = 102,
    TG_INTERNAL = 103,
    TG_READ_ONLY_WRITE = 104,
    TG_SYNTAX = 105,
    TG_IDENTIFIER_IS_NOT_NODE = 106,
    TG_IDENTIFIER_IS_NOT_EDGE = 107,
    TG_IDENTIGIER_EXISTS = 108,
    TG_UNKNOWN_IDENTIFIER = 109,
    TG_TYPE_MISMATCH = 110,
    TG_INDEX_OUT_OF_BOUNDS = 111,
    TG_MISSING_NODE = 112,
    TG_MISSING_EDGE = 113,
    TG_DELETE_CONNECTED = 114,

    // FFI specific errors
    TG_INVALID_STRING = 115,
    TG_OPEN_TRANSACTION = 116,
    TG_OPEN_STATEMENT = 117,
    TG_MISUSE = 118,
}

#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum TimesGraphType {
    TG_ID = 0,
    TG_INTEGER = 1,
    TG_REAL = 2,
    TG_BOOLEAN = 3,
    TG_TEXT = 4,
    TG_BLOB = 5,
    TG_NULL = 6,
}

pub struct TimesGraphStore {
    store: Store,
    txn_count: AtomicUsize,
    stmt_count: AtomicUsize,
}

pub struct TimesGraphTxn {
    graph: *const TimesGraphStore,
    txn: StoreTxn<'static>,
}

pub struct TimesGraphStatement {
    graph: *const TimesGraphStore,
    program: *mut Program,
    parameters: HashMap<String, PropOwned>,
    runtime: Option<(
        VirtualMachine<'static, 'static, 'static>,
        Vec<Option<Vec<u8>>>,
    )>,
}

#[no_mangle]
pub unsafe extern "C" fn tg_open(
    path: *const c_char,
    graph: *mut *mut TimesGraphStore,
) -> TimesGraphStatus {
    let inner = || -> Result<TimesGraphStore, TimesGraphStatus> {
        let path = CStr::from_ptr(path)
            .to_str()
            .map_err(|_| TimesGraphStatus::TG_INVALID_STRING)?;
        Ok(TimesGraphStore {
            store: Store::open(path)?,
            txn_count: AtomicUsize::new(0),
            stmt_count: AtomicUsize::new(0),
        })
    };
    match inner() {
        Err(err) => err,
        Ok(g) => {
            *graph = Box::into_raw(Box::new(g));
            TimesGraphStatus::TG_OK
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_open_anon(graph: *mut *mut TimesGraphStore) -> TimesGraphStatus {
    let inner = || -> Result<TimesGraphStore, TimesGraphStatus> {
        Ok(TimesGraphStore {
            store: Store::open_anon()?,
            txn_count: AtomicUsize::new(0),
            stmt_count: AtomicUsize::new(0),
        })
    };
    match inner() {
        Err(err) => err,
        Ok(g) => {
            *graph = Box::into_raw(Box::new(g));
            TimesGraphStatus::TG_OK
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_close(graph: *mut TimesGraphStore) -> TimesGraphStatus {
    if (*graph).txn_count.load(Ordering::SeqCst) > 0 {
        TimesGraphStatus::TG_OPEN_TRANSACTION
    } else if (*graph).stmt_count.load(Ordering::SeqCst) > 0 {
        TimesGraphStatus::TG_OPEN_STATEMENT
    } else {
        drop(Box::from_raw(graph));
        TimesGraphStatus::TG_OK
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_txn(
    graph: *const TimesGraphStore,
    txn: *mut *mut TimesGraphTxn,
) -> TimesGraphStatus {
    let inner = || -> Result<TimesGraphTxn, TimesGraphStatus> {
        let txn = (*graph).store.txn()?;
        (*graph).txn_count.fetch_add(1, Ordering::SeqCst);
        Ok(TimesGraphTxn { graph, txn })
    };
    match inner() {
        Err(err) => err,
        Ok(t) => {
            *txn = Box::into_raw(Box::new(t));
            TimesGraphStatus::TG_OK
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_mut_txn(
    graph: *const TimesGraphStore,
    txn: *mut *mut TimesGraphTxn,
) -> TimesGraphStatus {
    let inner = || -> Result<TimesGraphTxn, TimesGraphStatus> {
        let txn = (*graph).store.mut_txn()?;
        (*graph).txn_count.fetch_add(1, Ordering::SeqCst);
        Ok(TimesGraphTxn { graph, txn })
    };
    match inner() {
        Err(err) => err,
        Ok(t) => {
            *txn = Box::into_raw(Box::new(t));
            TimesGraphStatus::TG_OK
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_drop(txn: *mut TimesGraphTxn) -> TimesGraphStatus {
    if !txn.is_null() {
        (*(*txn).graph).txn_count.fetch_sub(1, Ordering::SeqCst);
        drop(Box::from_raw(txn));
    }
    TimesGraphStatus::TG_OK
}

#[no_mangle]
pub unsafe extern "C" fn tg_commit(txn: *mut TimesGraphTxn) -> TimesGraphStatus {
    let inner = || -> Result<(), TimesGraphStatus> {
        let txn = read(txn);
        txn.txn.commit()?;
        (*txn.graph).txn_count.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    };
    match inner() {
        Err(err) => err,
        Ok(()) => TimesGraphStatus::TG_OK,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_prepare(
    graph: *const TimesGraphStore,
    query: *const c_char,
    stmt: *mut *mut TimesGraphStatement,
) -> TimesGraphStatus {
    let inner = || -> Result<TimesGraphStatement, TimesGraphStatus> {
        // 获取查询语句
        let query = CStr::from_ptr(query)
            .to_str()
            .map_err(|_| TimesGraphStatus::TG_INVALID_STRING)?;
        let ast = parser::parse(query).map_err(|_| TimesGraphStatus::TG_SYNTAX)?;
        let plan = QueryPlan::new(&ast)?.optimize()?;
        let program = Box::into_raw(Box::new(Program::new(&plan)?));
        (*graph).stmt_count.fetch_add(1, Ordering::SeqCst);
        Ok(TimesGraphStatement {
            graph,
            program,
            parameters: HashMap::new(),
            runtime: None,
        })
    };
    match inner() {
        Err(err) => err,
        Ok(s) => {
            *stmt = Box::into_raw(Box::new(s));
            TimesGraphStatus::TG_OK
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_start(
    stmt: *mut TimesGraphStatement,
    txn: *mut TimesGraphTxn,
) -> TimesGraphStatus {
    (*stmt).runtime = Some((
        VirtualMachine::new(
            &mut txn.as_mut().unwrap().txn,
            (*stmt).program.as_mut().unwrap(),
            (*stmt).parameters.clone(),
        ),
        (*(*stmt).program).returns.iter().map(|_| None).collect(),
    ));
    TimesGraphStatus::TG_OK
}

#[no_mangle]
pub unsafe extern "C" fn tg_step(stmt: *mut TimesGraphStatement) -> TimesGraphStatus {
    if let Some((vm, buffers)) = (*stmt).runtime.as_mut() {
        let mut inner = || -> Result<TimesGraphStatus, TimesGraphStatus> {
            if (*(*stmt).program).returns.is_empty() {
                loop {
                    match vm.run()? {
                        Status::Yield => continue,
                        Status::Halt => break Ok(TimesGraphStatus::TG_DONE),
                    }
                }
            } else {
                buffers.iter_mut().for_each(|b| *b = None);
                match vm.run()? {
                    Status::Yield => Ok(TimesGraphStatus::TG_MATCH),
                    Status::Halt => Ok(TimesGraphStatus::TG_DONE),
                }
            }
        };
        match inner() {
            Err(err) => err,
            Ok(status) => status,
        }
    } else {
        TimesGraphStatus::TG_MISUSE
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_finalize(stmt: *mut TimesGraphStatement) -> TimesGraphStatus {
    if !stmt.is_null() {
        let stmt_count = &(*(*stmt).graph).stmt_count;
        drop(Box::from_raw((*stmt).program));
        drop(Box::from_raw(stmt));
        stmt_count.fetch_sub(1, Ordering::SeqCst);
    }
    TimesGraphStatus::TG_OK
}

#[no_mangle]
pub unsafe extern "C" fn tg_bind_id(
    stmt: *mut TimesGraphStatement,
    name: *const c_char,
    value: u64,
) -> TimesGraphStatus {
    let inner = || -> Result<(), TimesGraphStatus> {
        let name = CStr::from_ptr(name)
            .to_str()
            .map_err(|_| TimesGraphStatus::TG_INVALID_STRING)?;
        (*stmt)
            .parameters
            .insert(name.to_string(), PropOwned::Id(value));
        Ok(())
    };
    match inner() {
        Err(err) => err,
        Ok(()) => TimesGraphStatus::TG_OK,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_bind_integer(
    stmt: *mut TimesGraphStatement,
    name: *const c_char,
    value: i64,
) -> TimesGraphStatus {
    let inner = || -> Result<(), TimesGraphStatus> {
        let name = CStr::from_ptr(name)
            .to_str()
            .map_err(|_| TimesGraphStatus::TG_INVALID_STRING)?;
        (*stmt)
            .parameters
            .insert(name.to_string(), PropOwned::Integer(value));
        Ok(())
    };
    match inner() {
        Err(err) => err,
        Ok(()) => TimesGraphStatus::TG_OK,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_bind_real(
    stmt: *mut TimesGraphStatement,
    name: *const c_char,
    value: f64,
) -> TimesGraphStatus {
    let inner = || -> Result<(), TimesGraphStatus> {
        let name = CStr::from_ptr(name)
            .to_str()
            .map_err(|_| TimesGraphStatus::TG_INVALID_STRING)?;
        (*stmt)
            .parameters
            .insert(name.to_string(), PropOwned::Real(value));
        Ok(())
    };
    match inner() {
        Err(err) => err,
        Ok(()) => TimesGraphStatus::TG_OK,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_bind_boolean(
    stmt: *mut TimesGraphStatement,
    name: *const c_char,
    value: bool,
) -> TimesGraphStatus {
    let inner = || -> Result<(), TimesGraphStatus> {
        let name = CStr::from_ptr(name)
            .to_str()
            .map_err(|_| TimesGraphStatus::TimesGraph_INVALID_STRING)?;
        (*stmt)
            .parameters
            .insert(name.to_string(), PropOwned::Boolean(value));
        Ok(())
    };
    match inner() {
        Err(err) => err,
        Ok(()) => TimesGraphStatus::TG_OK,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_bind_text(
    stmt: *mut TimesGraphStatement,
    name: *const c_char,
    value: *const c_char,
) -> TimesGraphStatus {
    let inner = || -> Result<(), TimesGraphStatus> {
        let name = CStr::from_ptr(name)
            .to_str()
            .map_err(|_| TimesGraphStatus::TG_INVALID_STRING)?;
        let value = CStr::from_ptr(value)
            .to_str()
            .map_err(|_| TimesGraphStatus::TG_INVALID_STRING)?;
        (*stmt)
            .parameters
            .insert(name.to_string(), PropOwned::Text(value.to_string()));
        Ok(())
    };
    match inner() {
        Err(err) => err,
        Ok(()) => TimesGraphStatus::TG_OK,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_bind_blob(
    stmt: *mut TimesGraphStatement,
    name: *const c_char,
    value: *const c_void,
    length: usize,
) -> TimesGraphStatus {
    let inner = || -> Result<(), TimesGraphStatus> {
        let name = CStr::from_ptr(name)
            .to_str()
            .map_err(|_| TimesGraphStatus::TG_INVALID_STRING)?;
        let value = std::slice::from_raw_parts(value as *const u8, length);
        (*stmt)
            .parameters
            .insert(name.to_string(), PropOwned::Blob(value.to_vec()));
        Ok(())
    };
    match inner() {
        Err(err) => err,
        Ok(()) => TimesGraphStatus::TG_OK,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_bind_null(
    stmt: *mut TimesGraphStatement,
    name: *const c_char,
) -> TimesGraphStatus {
    let inner = || -> Result<(), TimesGraphStatus> {
        let name = CStr::from_ptr(name)
            .to_str()
            .map_err(|_| TimesGraphStatus::TG_INVALID_STRING)?;
        (*stmt).parameters.remove(name);
        Ok(())
    };
    match inner() {
        Err(err) => err,
        Ok(()) => TimesGraphStatus::TG_OK,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_return_count(stmt: *mut TimesGraphStatement) -> usize {
    (*(*stmt).program).returns.len()
}

#[no_mangle]
pub unsafe extern "C" fn tg_return_type(stmt: *mut TimesGraphStatement, idx: usize) -> TimesGraphType {
    let (vm, _) = (*stmt).runtime.as_mut().unwrap();
    match vm.access_return(idx).unwrap() {
        PropOwned::Id(_) => TimesGraphType::TG_ID,
        PropOwned::Integer(_) => TimesGraphType::TG_INTEGER,
        PropOwned::Real(_) => TimesGraphType::TG_REAL,
        PropOwned::Boolean(_) => TimesGraphType::TG_BOOLEAN,
        PropOwned::Text(_) => TimesGraphType::TG_TEXT,
        PropOwned::Blob(_) => TimesGraphType::TG_BLOB,
        PropOwned::Null => TimesGraphType::TG_NULL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_return_id(stmt: *mut TimesGraphStatement, idx: usize) -> u64 {
    let (vm, _) = (*stmt).runtime.as_mut().unwrap();
    match vm.access_return(idx).unwrap() {
        PropOwned::Id(id) => id,
        _ => panic!(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_return_integer(stmt: *mut TimesGraphStatement, idx: usize) -> i64 {
    let (vm, _) = (*stmt).runtime.as_mut().unwrap();
    match vm.access_return(idx).unwrap() {
        PropOwned::Integer(num) => num,
        _ => panic!(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_return_real(stmt: *mut TimesGraphStatement, idx: usize) -> f64 {
    let (vm, _) = (*stmt).runtime.as_mut().unwrap();
    match vm.access_return(idx).unwrap() {
        PropOwned::Real(num) => num,
        _ => panic!(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_return_boolean(stmt: *mut TimesGraphStatement, idx: usize) -> bool {
    let (vm, _) = (*stmt).runtime.as_mut().unwrap();
    match vm.access_return(idx).unwrap() {
        PropOwned::Boolean(val) => val,
        _ => panic!(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_return_text(
    stmt: *mut TimesGraphStatement,
    idx: usize,
) -> *const c_char {
    let (vm, buffers) = (*stmt).runtime.as_mut().unwrap();
    match &buffers[idx] {
        Some(buffer) => buffer.as_ptr() as *const c_char,
        None => match vm.access_return(idx).unwrap() {
            PropOwned::Text(string) => {
                let mut buf = string.into_bytes();
                buf.push(0);
                buffers[idx] = Some(buf);
                buffers[idx].as_ref().unwrap().as_ptr() as *const c_char
            }
            _ => panic!(),
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_return_blob(
    stmt: *mut TimesGraphStatement,
    idx: usize,
) -> *const c_void {
    let (vm, buffers) = (*stmt).runtime.as_mut().unwrap();
    match &buffers[idx] {
        Some(buffer) => buffer.as_ptr() as *const c_void,
        None => match vm.access_return(idx).unwrap() {
            PropOwned::Text(string) => {
                let buf = string.into_bytes();
                buffers[idx] = Some(buf);
                buffers[idx].as_ref().unwrap().as_ptr() as *const c_void
            }
            _ => panic!(),
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn tg_return_bytes(stmt: *mut TimesGraphStatement, idx: usize) -> usize {
    let (_, buffers) = (*stmt).runtime.as_mut().unwrap();
    match &buffers[idx] {
        Some(buffer) => buffer.len(),
        None => 0,
    }
}

impl From<Error> for TimesGraphStatus {
    fn from(err: Error) -> Self {
        match err {
            Error::IO(_) => TimesGraphStatus::TG_IO,
            Error::Corruption => TimesGraphStatus::TG_CORRUPTION,
            Error::Poison => TimesGraphStatus::TG_POISON,
            Error::Internal => TimesGraphStatus::TG_INTERNAL,
            Error::ReadOnlyWrite => TimesGraphStatus::TG_READ_ONLY_WRITE,
            Error::Syntax { .. } => TimesGraphStatus::TG_SYNTAX,
            Error::IdentifierIsNotNode(_) => TimesGraphStatus::TG_IDENTIFIER_IS_NOT_NODE,
            Error::IdentifierIsNotEdge(_) => TimesGraphStatus::TG_IDENTIFIER_IS_NOT_EDGE,
            Error::IdentifierExists(_) => TimesGraphStatus::TG_IDENTIGIER_EXISTS,
            Error::UnknownIdentifier(_) => TimesGraphStatus::TG_UNKNOWN_IDENTIFIER,
            Error::TypeMismatch => TimesGraphStatus::TG_TYPE_MISMATCH,
            Error::IndexOutOfBounds => TimesGraphStatus::TG_INDEX_OUT_OF_BOUNDS,
            Error::MissingNode => TimesGraphStatus::TG_MISSING_NODE,
            Error::MissingEdge => TimesGraphStatus::TG_MISSING_EDGE,
            Error::DeleteConnected => TimesGraphStatus::TG_DELETE_CONNECTED,
        }
    }
}
