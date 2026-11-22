//! End-to-end test of the *installed* IME, confined to this process:
//! activates the profile for this process only, types into its own window
//! with SendInput and checks the resulting text.
//!
//! TSF only routes keys to an IME on the foreground thread, so this needs the
//! desktop: don't touch the keyboard/mouse while it runs. If the window loses
//! the foreground it aborts immediately (it never types into other windows).
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::TextServices::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const CLSID: GUID = GUID::from_u128(0x17432efe_970e_4ba4_a40a_014c6c458922);
const PROFILE: GUID = GUID::from_u128(0x6e2baeeb_aa1c_4529_95d9_e8acb1b58162);

/// keys: lowercase = key, uppercase = shift+key, '<' = backspace, ' ' = space, '\n' = enter
const CASES: &[(&str, &str)] = &[
    ("dkssudgktpdy", "안녕하세요"),
    ("gksrmf dlqfur", "한글 입력"),
    ("ekfrdl djqtdj", "닭이 없어"),
    ("dlTek, Rkcl.", "있다, 까치."),
    ("ghkdnjsdml", "화원의"),
    ("gks<<rk", "ㅎ가"),
    ("gksrmf<<<", "한"),
    ("rkskek\nfk", "가나다\r\n라"),
    ("gks{rmf", "글한"),
    ("tkfkd!", "사랑"),
    ("QWERTOP", "ㅃㅉㄸㄲ썌ㅖ"),
];

fn pump() {
    unsafe {
        for _ in 0..30 {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            std::thread::sleep(std::time::Duration::from_millis(3));
        }
    }
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: unsafe { MapVirtualKeyW(vk.0 as u32, MAPVK_VK_TO_VSC) } as u16,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                ..Default::default()
            },
        },
    }
}

static mut TARGET: HWND = HWND(std::ptr::null_mut());

fn tap(vk: VIRTUAL_KEY, shift: bool) {
    tap_sendinput(vk, shift)
}

/// Real keyboard input path. Only used when our window is in the foreground.
fn tap_sendinput(vk: VIRTUAL_KEY, shift: bool) {
    unsafe {
        if GetForegroundWindow() != GetParent(TARGET).unwrap_or_default() {
            eprintln!("ABORT: test window is not in the foreground");
            std::process::exit(3);
        }
    }
    let mut v = vec![];
    if shift { v.push(key(VK_SHIFT, false)); }
    v.push(key(vk, false));
    v.push(key(vk, true));
    if shift { v.push(key(VK_SHIFT, true)); }
    unsafe { SendInput(&v, std::mem::size_of::<INPUT>() as i32) };
    pump();
}

fn type_str(s: &str) {
    for c in s.chars() {
        match c {
            '<' => tap(VK_BACK, false),
            ' ' => tap(VK_SPACE, false),
            '\n' => tap(VK_RETURN, false),
            ',' => tap(VK_OEM_COMMA, false),
            '.' => tap(VK_OEM_PERIOD, false),
            '{' => tap(VK_LEFT, false),
            '!' => tap(VK_ESCAPE, false),
            c if c.is_ascii_alphabetic() => tap(VIRTUAL_KEY(c.to_ascii_uppercase() as u16), c.is_ascii_uppercase()),
            _ => panic!("unsupported {c}"),
        }
    }
}

fn text(h: HWND) -> String {
    let mut buf = vec![0u16; 4096];
    let n = unsafe { GetWindowTextW(h, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

fn run(class: PCWSTR, label: &str) -> Result<usize> {
    unsafe {
        let hinst = GetModuleHandleW(None)?;
        let wnd = CreateWindowExW(WINDOW_EX_STYLE(0), w!("STATIC"), w!("hangul e2e"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE, 100, 100, 600, 300, None, None, Some(hinst.into()), None)?;
        let edit = CreateWindowExW(WINDOW_EX_STYLE(0), class, w!(""),
            WS_CHILD | WS_VISIBLE | WS_BORDER | WINDOW_STYLE((ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN) as u32),
            0, 0, 580, 260, Some(wnd), None, Some(hinst.into()), None)?;
        TARGET = edit;
        {
            let fg = GetForegroundWindow();
            let fg_thread = GetWindowThreadProcessId(fg, None);
            let me = windows::Win32::System::Threading::GetCurrentThreadId();
            let attached = fg_thread != 0 && fg_thread != me
                && windows::Win32::System::Threading::AttachThreadInput(me, fg_thread, true).as_bool();
            let _ = BringWindowToTop(wnd);
            let _ = SetForegroundWindow(wnd);
            if attached { let _ = windows::Win32::System::Threading::AttachThreadInput(me, fg_thread, false); }
        }
        let _ = SetForegroundWindow(wnd);
        let _ = SetFocus(Some(edit));
        pump();
        let mut fails = 0;
        for (keys, want) in CASES {
            let _ = SetWindowTextW(edit, w!(""));
            pump();
            type_str(keys);
            tap(VK_ESCAPE, false); // flushes any composition (commit-and-pass)
            let got = text(edit);
            let ok = got == *want;
            if !ok { fails += 1; }
            println!("[{label}] {} {:<16} -> {:?} (want {:?})", if ok { "OK  " } else { "FAIL" }, keys.replace('\n', "\n"), got, want);
        }
        let _ = DestroyWindow(wnd);
        pump();
        Ok(fails)
    }
}

fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let mgr: ITfInputProcessorProfileMgr =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        mgr.ActivateProfile(TF_PROFILETYPE_INPUTPROCESSOR, 0x0412, &CLSID, &PROFILE, HKL::default(),
            TF_IPPMF_FORPROCESS | TF_IPPMF_DONTCARECURRENTINPUTLANGUAGE)?;
        let mut prof = TF_INPUTPROCESSORPROFILE::default();
        mgr.GetActiveProfile(&GUID_TFCAT_TIP_KEYBOARD, &mut prof)?;
        println!("active profile clsid = {:?}", prof.clsid);
        if prof.clsid != CLSID { println!("FAIL: profile not active"); std::process::exit(2); }

        LoadLibraryW(w!("Msftedit.dll"))?;
        let mut fails = run(w!("EDIT"), "EDIT")?;
        fails += run(w!("RICHEDIT50W"), "RichEdit")?;
        println!("{}", if fails == 0 { "ALL PASSED".to_string() } else { format!("{fails} FAILED") });
        std::process::exit(if fails == 0 { 0 } else { 1 });
    }
}
