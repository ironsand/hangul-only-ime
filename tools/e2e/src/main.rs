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
// Microsoft Korean IME, for comparison runs (`e2e --ms`): only the composing cases are run and printed.
const MS_CLSID: GUID = GUID::from_u128(0xa028ae76_01b1_46c2_99c4_acd9858ae02f);
const MS_PROFILE: GUID = GUID::from_u128(0xb5fe1f02_d5f2_4445_9c03_c568f23c99a1);
static mut MS: bool = false;

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

/// Typed without committing: expected text and the UTF-16 offset of the composing syllable.
/// The expected selection is what Microsoft's Korean IME produces (`e2e --ms`): an EDIT control reports the caret
/// after the composing syllable, RichEdit reports it collapsed in front of it.
const COMPOSING: &[(&str, &str, u32)] = &[
    ("gk", "하", 0),
    ("gks", "한", 0),
    ("gksg", "핞", 0),
    ("gksgk", "한하", 1),
    ("gksrmf", "한글", 1),
    ("dkssud", "안녕", 1),
];

static mut SHOT_DIR: Option<String> = None;

/// Saves the on-screen pixels of `h` as a 24-bit BMP.
unsafe fn screenshot(h: HWND, path: &str) {
    use windows::Win32::Graphics::Gdi::*;
    let mut r = RECT::default();
    let _ = GetWindowRect(h, &mut r);
    let (w, ht) = (r.right - r.left, r.bottom - r.top);
    let screen = GetDC(None);
    let mem = CreateCompatibleDC(Some(screen));
    let bmp = CreateCompatibleBitmap(screen, w, ht);
    let old = SelectObject(mem, bmp.into());
    let _ = BitBlt(mem, 0, 0, w, ht, Some(screen), r.left, r.top, SRCCOPY);
    SelectObject(mem, old);
    let stride = ((w * 3 + 3) & !3) as usize;
    let mut pixels = vec![0u8; stride * ht as usize];
    let mut bi = BITMAPINFO::default();
    bi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: ht, biPlanes: 1, biBitCount: 24,
        ..Default::default()
    };
    GetDIBits(mem, bmp, 0, ht as u32, Some(pixels.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
    let _ = DeleteObject(bmp.into());
    let _ = DeleteDC(mem);
    ReleaseDC(None, screen);
    let mut f = Vec::new();
    let size = 54 + pixels.len() as u32;
    f.extend_from_slice(b"BM");
    f.extend_from_slice(&size.to_le_bytes());
    f.extend_from_slice(&[0; 4]);
    f.extend_from_slice(&54u32.to_le_bytes());
    f.extend_from_slice(&40u32.to_le_bytes());
    f.extend_from_slice(&w.to_le_bytes());
    f.extend_from_slice(&ht.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&24u16.to_le_bytes());
    f.extend_from_slice(&[0; 24]);
    f.extend_from_slice(&pixels);
    let _ = std::fs::write(path, f);
}

fn selection(h: HWND) -> (u32, u32) {
    const EM_GETSEL: u32 = 0x00B0;
    let (mut s, mut e) = (0u32, 0u32);
    unsafe {
        SendMessageW(h, EM_GETSEL, Some(WPARAM(&mut s as *mut u32 as usize)), Some(LPARAM(&mut e as *mut u32 as isize)));
    }
    (s, e)
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
        if label == "RichEdit-TSF" {
            // Make RichEdit a native TSF client (like Windows 11 Notepad) instead of going through the IMM32
            // compatibility layer, whose TSF contexts are "transitory".
            const EM_SETEDITSTYLE: u32 = 0x0400 + 204;
            const SES_USECTF: usize = 0x0001_0000;
            SendMessageW(edit, EM_SETEDITSTYLE, Some(WPARAM(SES_USECTF)), Some(LPARAM(SES_USECTF as isize)));
        }
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
        if MS {
            use windows::Win32::UI::Input::Ime::*;
            let himc = ImmGetContext(edit);
            let _ = ImmSetOpenStatus(himc, true);
            let _ = ImmSetConversionStatus(himc, IME_CMODE_HANGUL, IME_SMODE_NONE);
            let _ = ImmReleaseContext(edit, himc);
            pump();
        }
        let mut fails = 0;
        let shots = (*std::ptr::addr_of!(SHOT_DIR)).is_some();
        let cases: &[(&str, &str)] = if MS || shots { &[] } else { CASES };
        for (keys, want) in cases {
            let _ = SetWindowTextW(edit, w!(""));
            pump();
            type_str(keys);
            tap(VK_ESCAPE, false); // flushes any composition (commit-and-pass)
            let got = text(edit);
            let ok = got == *want;
            if !ok { fails += 1; }
            println!("[{label}] {} {:<16} -> {:?} (want {:?})", if ok { "OK  " } else { "FAIL" }, keys.replace('\n', "\n"), got, want);
        }
        // While composing, the composing syllable (the last one) must be the selected one.
        for (keys, want, sel) in COMPOSING {
            let _ = SetWindowTextW(edit, w!(""));
            pump();
            type_str(keys);
            let got = text(edit);
            let got_sel = selection(edit);
            if let Some(dir) = (*std::ptr::addr_of!(SHOT_DIR)).as_deref() {
                std::thread::sleep(std::time::Duration::from_millis(300));
                pump();
                let who = if MS { "ms" } else { "ours" };
                screenshot(edit, &format!("{dir}/{who}-{label}-{keys}.bmp"));
            }
            tap(VK_ESCAPE, false);
            let sel = if label == "RichEdit" { (*sel, *sel) } else { (*sel + 1, *sel + 1) };
            let ok = got == *want && got_sel == sel;
            if !ok { fails += 1; }
            println!("[{label}] {} {:<16} -> {:?} sel {:?} (want {:?} sel {:?}) [composing]",
                if ok { "OK  " } else { "FAIL" }, keys, got, got_sel, want, sel);
        }
        let _ = DestroyWindow(wnd);
        pump();
        Ok(fails)
    }
}

fn main() -> Result<()> {
    unsafe {
        // Real pixel coordinates, so screenshots line up with window rectangles on scaled displays.
        let _ = SetProcessDPIAware();
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let mgr: ITfInputProcessorProfileMgr =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        let args: Vec<String> = std::env::args().collect();
        MS = args.iter().any(|a| a == "--ms");
        SHOT_DIR = args.iter().position(|a| a == "--shot").and_then(|i| args.get(i + 1).cloned());
        let (clsid, profile) = if MS { (MS_CLSID, MS_PROFILE) } else { (CLSID, PROFILE) };
        mgr.ActivateProfile(TF_PROFILETYPE_INPUTPROCESSOR, 0x0412, &clsid, &profile, HKL::default(),
            TF_IPPMF_FORPROCESS | TF_IPPMF_DONTCARECURRENTINPUTLANGUAGE)?;
        let mut prof = TF_INPUTPROCESSORPROFILE::default();
        mgr.GetActiveProfile(&GUID_TFCAT_TIP_KEYBOARD, &mut prof)?;
        println!("active profile clsid = {:?}", prof.clsid);
        if prof.clsid != clsid { println!("FAIL: profile not active"); std::process::exit(2); }

        LoadLibraryW(w!("Msftedit.dll"))?;
        let mut fails = run(w!("EDIT"), "EDIT")?;
        fails += run(w!("RICHEDIT50W"), "RichEdit")?;
        fails += run(w!("RICHEDIT50W"), "RichEdit-TSF")?;
        // RichEditD2DPT (used by Windows 11 Notepad) is not registered outside Notepad, so it cannot be tested here.
        println!("{}", if fails == 0 { "ALL PASSED".to_string() } else { format!("{fails} FAILED") });
        std::process::exit(if fails == 0 { 0 } else { 1 });
    }
}
