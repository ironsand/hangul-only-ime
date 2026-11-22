//! Loads hangul_ime.dll without registering it and exercises the COM surface.
use std::ffi::c_void;
use windows::core::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::TextServices::*;

const CLSID: GUID = GUID::from_u128(0x17432efe_970e_4ba4_a40a_014c6c458922);
type GetClassObject = unsafe extern "system" fn(*const GUID, *const GUID, *mut *mut c_void) -> HRESULT;

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("usage: smoke <dll>");
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let w: Vec<u16> = path.encode_utf16().chain([0]).collect();
        let h = LoadLibraryW(PCWSTR(w.as_ptr()))?;
        let f: GetClassObject = std::mem::transmute(GetProcAddress(h, s!("DllGetClassObject")).unwrap());
        let mut p = std::ptr::null_mut();
        f(&CLSID, &IClassFactory::IID, &mut p).ok()?;
        let cf = IClassFactory::from_raw(p);
        let tip: ITfTextInputProcessor = cf.CreateInstance(None)?;
        println!("CreateInstance ok");
        let _: ITfTextInputProcessorEx = tip.cast()?;
        let _: ITfKeyEventSink = tip.cast()?;
        let _: ITfCompositionSink = tip.cast()?;
        let dap: ITfDisplayAttributeProvider = tip.cast()?;
        println!("QueryInterface ok (Ex, KeyEventSink, CompositionSink, DisplayAttributeProvider)");
        let en = dap.EnumDisplayAttributeInfo()?;
        let mut items = [None];
        let mut n = 0;
        let _ = en.Next(&mut items, &mut n);
        println!("display attrs enumerated: {n}, guid={:?}", items[0].as_ref().unwrap().GetGUID()?);

        // Wrong CLSID must be rejected.
        let bad = f(&GUID::zeroed(), &IClassFactory::IID, &mut p);
        println!("bad clsid -> {:?}", bad);

        // Activate against a real thread manager.
        let tm: ITfThreadMgr = CoCreateInstance(&CLSID_TF_ThreadMgr, None, CLSCTX_INPROC_SERVER)?;
        let tid = tm.Activate()?;
        // Without registration TSF rejects the key sink (E_INVALIDARG); that is expected.
        println!("Activate (unregistered) -> {:?}", tip.Activate(&tm, tid).map_err(|e| e.code()));
        tip.Deactivate()?;
        tip.Deactivate()?; // idempotent
        println!("Deactivate ok");
        tm.Deactivate()?;
        println!("ALL OK");
    }
    Ok(())
}
