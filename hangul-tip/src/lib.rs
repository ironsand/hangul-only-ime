//! Hangul IME — TSF text input processor (COM in-proc server).

mod display_attr;
pub mod log;
mod register;
mod service;

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

pub const CLSID_HANGUL_IME: GUID = GUID::from_u128(0x17432efe_970e_4ba4_a40a_014c6c458922);
pub const GUID_PROFILE: GUID = GUID::from_u128(0x6e2baeeb_aa1c_4529_95d9_e8acb1b58162);
pub const GUID_DISPLAY_ATTR_INPUT: GUID = GUID::from_u128(0x679f9316_d3e6_4561_a25a_280a39c1aea6);
pub const LANGID_KOREAN: u16 = 0x0412;
pub const IME_NAME: &str = "Hangul IME (두벌식)";

static DLL_INSTANCE: AtomicUsize = AtomicUsize::new(0);

pub fn dll_instance() -> HMODULE {
    HMODULE(DLL_INSTANCE.load(Ordering::Relaxed) as *mut c_void)
}

/// Runs `f`, turning a panic into an error so it never unwinds into the host app.
pub fn guard<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(_) => Err(E_UNEXPECTED.into()),
    }
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

#[no_mangle]
extern "system" fn DllMain(hinst: HMODULE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        DLL_INSTANCE.store(hinst.0 as usize, Ordering::Relaxed);
    }
    TRUE
}

#[implement(IClassFactory)]
struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> Result<()> {
        guard(|| unsafe {
            if ppv.is_null() || riid.is_null() {
                return Err(E_POINTER.into());
            }
            *ppv = std::ptr::null_mut();
            if outer.is_some() {
                return Err(CLASS_E_NOAGGREGATION.into());
            }
            let unk: IUnknown = service::TextService::create();
            unk.query(riid, ppv).ok()
        })
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

#[no_mangle]
extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    let r = guard(|| unsafe {
        if ppv.is_null() || rclsid.is_null() || riid.is_null() {
            return Err(E_POINTER.into());
        }
        *ppv = std::ptr::null_mut();
        if *rclsid != CLSID_HANGUL_IME {
            return Err(CLASS_E_CLASSNOTAVAILABLE.into());
        }
        let factory: IClassFactory = ClassFactory.into();
        factory.query(riid, ppv).ok()
    });
    match r {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    }
}

/// Never unload: the DLL lives in every process that types text, and staying
/// loaded removes a whole class of use-after-unload crashes.
#[no_mangle]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

#[no_mangle]
extern "system" fn DllRegisterServer() -> HRESULT {
    match guard(register::register) {
        Ok(()) => S_OK,
        Err(e) => {
            let _ = guard(register::unregister);
            e.code()
        }
    }
}

#[no_mangle]
extern "system" fn DllUnregisterServer() -> HRESULT {
    match guard(register::unregister) {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    }
}
