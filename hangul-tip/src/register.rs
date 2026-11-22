//! COM + TSF registration (called from regsvr32; needs administrator rights).

use windows::core::*;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows::Win32::UI::TextServices::*;

use crate::{dll_instance, wide, CLSID_HANGUL_IME, GUID_PROFILE, IME_NAME, LANGID_KOREAN};

const CATEGORIES: [GUID; 3] = [
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
];

fn clsid_key() -> String {
    format!("CLSID\\{{{:?}}}", CLSID_HANGUL_IME)
}

fn module_path() -> Result<String> {
    let mut buf = vec![0u16; 1024];
    let n = unsafe { GetModuleFileNameW(Some(dll_instance()), &mut buf) } as usize;
    if n == 0 || n >= buf.len() {
        return Err(Error::from_thread());
    }
    Ok(String::from_utf16_lossy(&buf[..n]))
}

fn set_string(key: HKEY, name: Option<&str>, value: &str) -> Result<()> {
    let mut data: Vec<u16> = wide(value);
    data.push(0);
    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2) };
    let name_w = name.map(|n| {
        let mut v = wide(n);
        v.push(0);
        v
    });
    let pname = match &name_w {
        Some(v) => PCWSTR(v.as_ptr()),
        None => PCWSTR::null(),
    };
    unsafe { RegSetValueExW(key, pname, None, REG_SZ, Some(bytes)).ok() }
}

fn create_key(parent: HKEY, path: &str) -> Result<HKEY> {
    let mut p = wide(path);
    p.push(0);
    let mut key = HKEY::default();
    unsafe {
        RegCreateKeyExW(
            parent,
            PCWSTR(p.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        )
        .ok()?;
    }
    Ok(key)
}

pub fn register() -> Result<()> {
    let path = module_path()?;

    // COM server registration.
    let key = create_key(HKEY_CLASSES_ROOT, &clsid_key())?;
    let r = set_string(key, None, IME_NAME);
    unsafe {
        let _ = RegCloseKey(key);
    }
    r?;
    let key = create_key(HKEY_CLASSES_ROOT, &format!("{}\\InprocServer32", clsid_key()))?;
    let r = set_string(key, None, &path).and_then(|_| set_string(key, Some("ThreadingModel"), "Apartment"));
    unsafe {
        let _ = RegCloseKey(key);
    }
    r?;

    unsafe {
        // Input profile under Korean.
        let mgr: ITfInputProcessorProfileMgr =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        mgr.RegisterProfile(
            &CLSID_HANGUL_IME,
            LANGID_KOREAN,
            &GUID_PROFILE,
            &wide(IME_NAME),
            &wide(""),
            0,
            HKL::default(),
            0,
            true,
            0,
        )?;

        let cat: ITfCategoryMgr = CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
        for c in CATEGORIES {
            cat.RegisterCategory(&CLSID_HANGUL_IME, &c, &CLSID_HANGUL_IME)?;
        }
    }
    Ok(())
}

pub fn unregister() -> Result<()> {
    unsafe {
        if let Ok(cat) = CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER) {
            for c in CATEGORIES {
                let _ = cat.UnregisterCategory(&CLSID_HANGUL_IME, &c, &CLSID_HANGUL_IME);
            }
        }
        if let Ok(mgr) =
            CoCreateInstance::<_, ITfInputProcessorProfileMgr>(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)
        {
            let _ = mgr.UnregisterProfile(&CLSID_HANGUL_IME, LANGID_KOREAN, &GUID_PROFILE, 0);
        }
        let mut p = wide(&clsid_key());
        p.push(0);
        let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(p.as_ptr()));
    }
    Ok(())
}
