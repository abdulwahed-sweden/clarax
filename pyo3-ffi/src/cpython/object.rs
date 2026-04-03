use crate::vectorcallfunc;
use crate::{object, PyGetSetDef, PyMemberDef, PyMethodDef, PyObject, Py_ssize_t};
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::mem;

// skipped private _Py_NewReference
// skipped private _Py_NewReferenceNoTotal
// skipped private _Py_ResurrectReference

// skipped private _Py_GetGlobalRefTotal
// skipped private _Py_GetRefTotal
// skipped private _Py_GetLegacyRefTotal
// skipped private _PyInterpreterState_GetRefTotal

// skipped private _Py_Identifier

// skipped private _Py_static_string_init
// skipped private _Py_static_string
// skipped private _Py_IDENTIFIER

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PyNumberMethods {
    pub nb_add: Option<object::binaryfunc>,
    pub nb_subtract: Option<object::binaryfunc>,
    pub nb_multiply: Option<object::binaryfunc>,
    pub nb_remainder: Option<object::binaryfunc>,
    pub nb_divmod: Option<object::binaryfunc>,
    pub nb_power: Option<object::ternaryfunc>,
    pub nb_negative: Option<object::unaryfunc>,
    pub nb_positive: Option<object::unaryfunc>,
    pub nb_absolute: Option<object::unaryfunc>,
    pub nb_bool: Option<object::inquiry>,
    pub nb_invert: Option<object::unaryfunc>,
    pub nb_lshift: Option<object::binaryfunc>,
    pub nb_rshift: Option<object::binaryfunc>,
    pub nb_and: Option<object::binaryfunc>,
    pub nb_xor: Option<object::binaryfunc>,
    pub nb_or: Option<object::binaryfunc>,
    pub nb_int: Option<object::unaryfunc>,
    pub nb_reserved: *mut c_void,
    pub nb_float: Option<object::unaryfunc>,
    pub nb_inplace_add: Option<object::binaryfunc>,
    pub nb_inplace_subtract: Option<object::binaryfunc>,
    pub nb_inplace_multiply: Option<object::binaryfunc>,
    pub nb_inplace_remainder: Option<object::binaryfunc>,
    pub nb_inplace_power: Option<object::ternaryfunc>,
    pub nb_inplace_lshift: Option<object::binaryfunc>,
    pub nb_inplace_rshift: Option<object::binaryfunc>,
    pub nb_inplace_and: Option<object::binaryfunc>,
    pub nb_inplace_xor: Option<object::binaryfunc>,
    pub nb_inplace_or: Option<object::binaryfunc>,
    pub nb_floor_divide: Option<object::binaryfunc>,
    pub nb_true_divide: Option<object::binaryfunc>,
    pub nb_inplace_floor_divide: Option<object::binaryfunc>,
    pub nb_inplace_true_divide: Option<object::binaryfunc>,
    pub nb_index: Option<object::unaryfunc>,
    pub nb_matrix_multiply: Option<object::binaryfunc>,
    pub nb_inplace_matrix_multiply: Option<object::binaryfunc>,
}

#[repr(C)]
#[derive(Clone)]
pub struct PySequenceMethods {
    pub sq_length: Option<object::lenfunc>,
    pub sq_concat: Option<object::binaryfunc>,
    pub sq_repeat: Option<object::ssizeargfunc>,
    pub sq_item: Option<object::ssizeargfunc>,
    pub was_sq_slice: *mut c_void,
    pub sq_ass_item: Option<object::ssizeobjargproc>,
    pub was_sq_ass_slice: *mut c_void,
    pub sq_contains: Option<object::objobjproc>,
    pub sq_inplace_concat: Option<object::binaryfunc>,
    pub sq_inplace_repeat: Option<object::ssizeargfunc>,
}

#[repr(C)]
#[derive(Clone, Default)]
pub struct PyMappingMethods {
    pub mp_length: Option<object::lenfunc>,
    pub mp_subscript: Option<object::binaryfunc>,
    pub mp_ass_subscript: Option<object::objobjargproc>,
}

#[cfg(Py_3_10)]
pub type sendfunc = unsafe extern "C" fn(
    iter: *mut PyObject,
    value: *mut PyObject,
    result: *mut *mut PyObject,
) -> object::PySendResult;

#[repr(C)]
#[derive(Clone, Default)]
pub struct PyAsyncMethods {
    pub am_await: Option<object::unaryfunc>,
    pub am_aiter: Option<object::unaryfunc>,
    pub am_anext: Option<object::unaryfunc>,
    #[cfg(Py_3_10)]
    pub am_send: Option<sendfunc>,
}

#[repr(C)]
#[derive(Clone, Default)]
pub struct PyBufferProcs {
    pub bf_getbuffer: Option<crate::getbufferproc>,
    pub bf_releasebuffer: Option<crate::releasebufferproc>,
}

pub type printfunc =
    unsafe extern "C" fn(arg1: *mut PyObject, arg2: *mut ::libc::FILE, arg3: c_int) -> c_int;

#[repr(C)]
#[derive(Debug)]
pub struct PyTypeObject {
    pub ob_base: object::PyVarObject,
    pub tp_name: *const c_char,
    pub tp_basicsize: Py_ssize_t,
    pub tp_itemsize: Py_ssize_t,
    pub tp_dealloc: Option<object::destructor>,
    pub tp_vectorcall_offset: Py_ssize_t,
    pub tp_getattr: Option<object::getattrfunc>,
    pub tp_setattr: Option<object::setattrfunc>,
    pub tp_as_async: *mut PyAsyncMethods,
    pub tp_repr: Option<object::reprfunc>,
    pub tp_as_number: *mut PyNumberMethods,
    pub tp_as_sequence: *mut PySequenceMethods,
    pub tp_as_mapping: *mut PyMappingMethods,
    pub tp_hash: Option<object::hashfunc>,
    pub tp_call: Option<object::ternaryfunc>,
    pub tp_str: Option<object::reprfunc>,
    pub tp_getattro: Option<object::getattrofunc>,
    pub tp_setattro: Option<object::setattrofunc>,
    pub tp_as_buffer: *mut PyBufferProcs,
    #[cfg(not(Py_GIL_DISABLED))]
    pub tp_flags: std::ffi::c_ulong,
    #[cfg(Py_GIL_DISABLED)]
    pub tp_flags: crate::impl_::AtomicCULong,
    pub tp_doc: *const c_char,
    pub tp_traverse: Option<object::traverseproc>,
    pub tp_clear: Option<object::inquiry>,
    pub tp_richcompare: Option<object::richcmpfunc>,
    pub tp_weaklistoffset: Py_ssize_t,
    pub tp_iter: Option<object::getiterfunc>,
    pub tp_iternext: Option<object::iternextfunc>,
    pub tp_methods: *mut PyMethodDef,
    pub tp_members: *mut PyMemberDef,
    pub tp_getset: *mut PyGetSetDef,
    pub tp_base: *mut PyTypeObject,
    pub tp_dict: *mut object::PyObject,
    pub tp_descr_get: Option<object::descrgetfunc>,
    pub tp_descr_set: Option<object::descrsetfunc>,
    pub tp_dictoffset: Py_ssize_t,
    pub tp_init: Option<object::initproc>,
    pub tp_alloc: Option<object::allocfunc>,
    pub tp_new: Option<object::newfunc>,
    pub tp_free: Option<object::freefunc>,
    pub tp_is_gc: Option<object::inquiry>,
    pub tp_bases: *mut object::PyObject,
    pub tp_mro: *mut object::PyObject,
    pub tp_cache: *mut object::PyObject,
    pub tp_subclasses: *mut object::PyObject,
    pub tp_weaklist: *mut object::PyObject,
    pub tp_del: Option<object::destructor>,
    pub tp_version_tag: c_uint,
    pub tp_finalize: Option<object::destructor>,
    pub tp_vectorcall: Option<vectorcallfunc>,
    #[cfg(Py_3_12)]
    pub tp_watched: c_char,
    #[cfg(py_sys_config = "COUNT_ALLOCS")]
    pub tp_allocs: Py_ssize_t,
    #[cfg(py_sys_config = "COUNT_ALLOCS")]
    pub tp_frees: Py_ssize_t,
    #[cfg(py_sys_config = "COUNT_ALLOCS")]
    pub tp_maxalloc: Py_ssize_t,
    #[cfg(py_sys_config = "COUNT_ALLOCS")]
    pub tp_prev: *mut PyTypeObject,
    #[cfg(py_sys_config = "COUNT_ALLOCS")]
    pub tp_next: *mut PyTypeObject,
    #[cfg(Py_3_13)]
    pub tp_versions_used: u16,
}

#[cfg(Py_3_11)]
#[repr(C)]
#[derive(Clone)]
struct _specialization_cache {
    getitem: *mut PyObject,
    #[cfg(Py_3_12)]
    getitem_version: u32,
    #[cfg(Py_3_13)]
    init: *mut PyObject,
}

#[repr(C)]
pub struct PyHeapTypeObject {
    pub ht_type: PyTypeObject,
    pub as_async: PyAsyncMethods,
    pub as_number: PyNumberMethods,
    pub as_mapping: PyMappingMethods,
    pub as_sequence: PySequenceMethods,
    pub as_buffer: PyBufferProcs,
    pub ht_name: *mut object::PyObject,
    pub ht_slots: *mut object::PyObject,
    pub ht_qualname: *mut object::PyObject,
    #[cfg(not(PyPy))]
    pub ht_cached_keys: *mut c_void,
    pub ht_module: *mut object::PyObject,
    #[cfg(all(Py_3_11, not(PyPy)))]
    _ht_tpname: *mut c_char,
    #[cfg(Py_3_14)]
    pub ht_token: *mut c_void,
    #[cfg(all(Py_3_11, not(PyPy)))]
    _spec_cache: _specialization_cache,
    #[cfg(all(Py_GIL_DISABLED, Py_3_14))]
    pub unique_id: Py_ssize_t,
}

impl Default for PyHeapTypeObject {
    #[inline]
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

// skipped private _PyType_Name
// skipped private _PyType_Lookup
// skipped private _PyType_LookupRef

extern_libpython! {
    #[cfg(Py_3_12)]
    pub fn PyType_GetDict(o: *mut PyTypeObject) -> *mut PyObject;

    #[cfg_attr(PyPy, link_name = "PyPyObject_Print")]
    pub fn PyObject_Print(o: *mut PyObject, fp: *mut ::libc::FILE, flags: c_int) -> c_int;

    // skipped private _Py_BreakPoint
    // skipped private _PyObject_Dump

    // skipped _PyObject_GetAttrId

    // skipped private _PyObject_GetDictPtr
    pub fn PyObject_CallFinalizer(arg1: *mut PyObject);
    #[cfg_attr(PyPy, link_name = "PyPyObject_CallFinalizerFromDealloc")]
    pub fn PyObject_CallFinalizerFromDealloc(arg1: *mut PyObject) -> c_int;

    // skipped private _PyObject_GenericGetAttrWithDict
    // skipped private _PyObject_GenericSetAttrWithDict
    // skipped private _PyObject_FunctionStr
}

// skipped Py_SETREF
// skipped Py_XSETREF

// skipped private _PyObject_ASSERT_FROM
// skipped private _PyObject_ASSERT_WITH_MSG
// skipped private _PyObject_ASSERT
// skipped private _PyObject_ASSERT_FAILED_MSG
// skipped private _PyObject_AssertFailed

// skipped private _PyTrash_begin
// skipped private _PyTrash_end

// skipped _PyTrash_thread_deposit_object
// skipped _PyTrash_thread_destroy_chain

// skipped Py_TRASHCAN_BEGIN
// skipped Py_TRASHCAN_END

// skipped PyObject_GetItemData

// skipped PyObject_VisitManagedDict
// skipped _PyObject_SetManagedDict
// skipped PyObject_ClearManagedDict

// skipped TYPE_MAX_WATCHERS

// skipped PyType_WatchCallback
// skipped PyType_AddWatcher
// skipped PyType_ClearWatcher
// skipped PyType_Watch
// skipped PyType_Unwatch

// skipped PyUnstable_Type_AssignVersionTag

// skipped PyRefTracerEvent

// skipped PyRefTracer
// skipped PyRefTracer_SetTracer
// skipped PyRefTracer_GetTracer

#[cfg(Py_3_14)]
extern_libpython! {
    // skipped PyUnstable_Object_EnableDeferredRefcount

    pub fn PyUnstable_Object_IsUniqueReferencedTemporary(obj: *mut PyObject) -> c_int;

    // skipped PyUnstable_IsImmortal

    pub fn PyUnstable_TryIncRef(obj: *mut PyObject) -> c_int;

    pub fn PyUnstable_EnableTryIncRef(obj: *mut PyObject) -> c_void;

    pub fn PyUnstable_Object_IsUniquelyReferenced(op: *mut PyObject) -> c_int;
}

#[cfg(Py_3_15)]
extern_libpython! {
    pub fn PyUnstable_SetImmortal(op: *mut PyObject) -> c_int;
}
