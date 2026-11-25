//! The text service: key handling and composition management.

use std::cell::{Cell, RefCell};
use std::mem::ManuallyDrop;

use hangul_core::{dubeolsik, Composer};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::TextServices::*;

use crate::display_attr::{DisplayAttributeInfo, EnumDisplayAttributeInfo};
use crate::{dlog, guard, wide, GUID_DISPLAY_ATTR_INPUT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    /// Not ours; let the application handle it.
    Pass,
    Jamo(char),
    Backspace,
    /// Commit what is being composed, then let the application handle the key.
    CommitAndPass,
}

#[implement(
    ITfTextInputProcessorEx,
    ITfKeyEventSink,
    ITfCompositionSink,
    ITfCompartmentEventSink,
    ITfDisplayAttributeProvider
)]
pub struct TextService {
    thread_mgr: RefCell<Option<ITfThreadMgr>>,
    client_id: Cell<u32>,
    composer: RefCell<Composer>,
    /// Committed text waiting for the next edit session to write it.
    pending: RefCell<String>,
    composition: RefCell<Option<ITfComposition>>,
    composition_ctx: RefCell<Option<ITfContext>>,
    attr_atom: Cell<u32>,
    /// Cookie of our sink on the keyboard open/close compartment.
    openclose_cookie: Cell<u32>,
}

/// TSF only routes keys to a keyboard TIP while the keyboard is "open".
/// This IME has no alphanumeric mode, so it keeps the keyboard open.
fn openclose_compartment(tm: &ITfThreadMgr) -> Result<ITfCompartment> {
    unsafe { tm.cast::<ITfCompartmentMgr>()?.GetCompartment(&GUID_COMPARTMENT_KEYBOARD_OPENCLOSE) }
}

fn ensure_open(tm: &ITfThreadMgr, tid: u32) -> Result<()> {
    unsafe {
        let c = openclose_compartment(tm)?;
        let open = c.GetValue().ok().and_then(|v| i32::try_from(&v).ok()).unwrap_or(0) != 0;
        if !open {
            c.SetValue(tid, &VARIANT::from(1i32))?;
        }
    }
    Ok(())
}

impl TextService {
    pub fn create() -> IUnknown {
        TextService {
            thread_mgr: RefCell::new(None),
            client_id: Cell::new(0),
            composer: RefCell::new(Composer::new()),
            pending: RefCell::new(String::new()),
            composition: RefCell::new(None),
            composition_ctx: RefCell::new(None),
            attr_atom: Cell::new(0),
            openclose_cookie: Cell::new(TF_INVALID_COOKIE),
        }
        .into()
    }

    fn is_composing(&self) -> bool {
        !self.composer.borrow().is_empty()
    }

    fn classify(&self, vk: u16) -> Action {
        let down = |k: VIRTUAL_KEY| unsafe { GetKeyState(k.0 as i32) } < 0;
        let shift = down(VK_SHIFT);
        let ctrl = down(VK_CONTROL);
        let alt = down(VK_MENU);
        let composing = self.is_composing();
        let vk = VIRTUAL_KEY(vk);

        if matches!(
            vk,
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT | VK_CONTROL | VK_LCONTROL | VK_RCONTROL | VK_MENU
                | VK_LMENU | VK_RMENU | VK_CAPITAL | VK_LWIN | VK_RWIN | VK_HANJA
        ) {
            return Action::Pass;
        }
        let other = if composing { Action::CommitAndPass } else { Action::Pass };
        if ctrl || alt {
            return other;
        }
        if (u16::from(b'A')..=u16::from(b'Z')).contains(&vk.0) {
            if let Some(j) = dubeolsik(vk.0 as u8 as char, shift) {
                return Action::Jamo(j);
            }
        }
        if vk == VK_BACK && composing {
            return Action::Backspace;
        }
        other
    }
}

/// Edit session running an arbitrary closure with the edit cookie.
#[implement(ITfEditSession)]
struct EditSession {
    f: RefCell<Option<Box<dyn FnOnce(u32) -> Result<()>>>>,
}

impl ITfEditSession_Impl for EditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        let f = self.f.borrow_mut().take();
        match f {
            Some(f) => guard(|| f(ec)),
            None => Ok(()),
        }
    }
}

/// Asks TSF for a write lock and then syncs the document with the engine state.
fn request_update(this: &ComObject<TextService>, ctx: &ITfContext) -> bool {
    request_update_with(this, ctx, TF_ES_ASYNCDONTCARE | TF_ES_READWRITE)
}

fn request_update_with(this: &ComObject<TextService>, ctx: &ITfContext, flags: TF_CONTEXT_EDIT_CONTEXT_FLAGS) -> bool {
    let me = this.clone();
    let c = ctx.clone();
    let session: ITfEditSession = EditSession {
        f: RefCell::new(Some(Box::new(move |ec| apply(&me, ec, &c)))),
    }
    .into();
    match unsafe { ctx.RequestEditSession(this.client_id.get(), &session, flags) } {
        Ok(hr) => hr.is_ok(),
        Err(_) => false,
    }
}

/// Writes pending committed text and the current preedit into the document.
fn apply(this: &ComObject<TextService>, ec: u32, ctx: &ITfContext) -> Result<()> {
    let commit = std::mem::take(&mut *this.pending.borrow_mut());
    let preedit = this.composer.borrow().preedit();
    let existing = this.composition.borrow().clone();
    dlog!("apply ec={ec} commit={commit:?} preedit={preedit:?} has_comp={}", existing.is_some());

    let comp = match existing {
        Some(c) => c,
        None => {
            if commit.is_empty() && preedit.is_empty() {
                return Ok(());
            }
            start_composition(this, ec, ctx)?
        }
    };
    let range = unsafe { comp.GetRange()? };

    if !commit.is_empty() {
        set_text(ec, &range, &commit)?;
        clear_attr(ec, ctx, &range);
        set_caret(ec, ctx, &range)?;
        end_composition(this, ec, &comp);
        if !preedit.is_empty() {
            let comp2 = start_composition(this, ec, ctx)?;
            let r2 = unsafe { comp2.GetRange()? };
            set_text(ec, &r2, &preedit)?;
            set_attr(this, ec, ctx, &r2);
            show_composing(ec, ctx, &r2)?;
        }
    } else if preedit.is_empty() {
        set_text(ec, &range, "")?;
        set_caret(ec, ctx, &range)?;
        end_composition(this, ec, &comp);
    } else {
        set_text(ec, &range, &preedit)?;
        set_attr(this, ec, ctx, &range);
        show_composing(ec, ctx, &range)?;
    }
    Ok(())
}

fn start_composition(this: &ComObject<TextService>, ec: u32, ctx: &ITfContext) -> Result<ITfComposition> {
    unsafe {
        let ins: ITfInsertAtSelection = ctx.cast()?;
        let range = ins.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
        let cc: ITfContextComposition = ctx.cast()?;
        let sink: ITfCompositionSink = this.to_interface();
        let comp = cc.StartComposition(ec, &range, &sink)?;
        *this.composition.borrow_mut() = Some(comp.clone());
        *this.composition_ctx.borrow_mut() = Some(ctx.clone());
        Ok(comp)
    }
}

fn end_composition(this: &ComObject<TextService>, ec: u32, comp: &ITfComposition) {
    *this.composition.borrow_mut() = None;
    *this.composition_ctx.borrow_mut() = None;
    unsafe {
        let _ = comp.EndComposition(ec);
    }
}

fn set_text(ec: u32, range: &ITfRange, s: &str) -> Result<()> {
    unsafe { range.SetText(ec, 0, &wide(s)) }
}

/// Collapses the selection to the end of `range` (after committed text).
fn set_caret(ec: u32, ctx: &ITfContext, range: &ITfRange) -> Result<()> {
    unsafe {
        let r = range.Clone()?;
        r.Collapse(ec, TF_ANCHOR_END)?;
        select(ec, ctx, r, false)
    }
}

/// Shows the composing syllable the way Microsoft's Korean IME does:
/// - In apps that reach TSF through the IMM32 compatibility layer (transitory contexts: classic EDIT and RichEdit
///   controls, ...), select it as an "interim character". Those apps treat a caret placed after the composition as
///   final and terminate it.
/// - In native TSF apps (Windows 11 Notepad, ...), put the caret after it. They draw an interim selection as a
///   highlight, and Microsoft's IME shows only the underline there.
fn show_composing(ec: u32, ctx: &ITfContext, range: &ITfRange) -> Result<()> {
    if is_transitory(ctx) {
        unsafe { select(ec, ctx, range.Clone()?, true) }
    } else {
        set_caret(ec, ctx, range)
    }
}

fn is_transitory(ctx: &ITfContext) -> bool {
    match unsafe { ctx.GetStatus() } {
        Ok(s) => s.dwStaticFlags & TF_SS_TRANSITORY != 0,
        // Unknown: the interim style is the one that keeps the composition alive everywhere.
        Err(_) => true,
    }
}

fn select(ec: u32, ctx: &ITfContext, r: ITfRange, interim: bool) -> Result<()> {
    unsafe {
        let mut sel = [TF_SELECTION {
            range: ManuallyDrop::new(Some(r)),
            style: TF_SELECTIONSTYLE { ase: TF_AE_NONE, fInterimChar: interim.into() },
        }];
        let res = ctx.SetSelection(ec, &sel);
        ManuallyDrop::drop(&mut sel[0].range);
        res
    }
}

fn set_attr(this: &ComObject<TextService>, ec: u32, ctx: &ITfContext, range: &ITfRange) {
    let atom = this.attr_atom.get();
    if atom == 0 {
        return;
    }
    unsafe {
        if let Ok(prop) = ctx.GetProperty(&GUID_PROP_ATTRIBUTE) {
            let v = VARIANT::from(atom as i32);
            let _ = prop.SetValue(ec, range, &v);
        }
    }
}

fn clear_attr(ec: u32, ctx: &ITfContext, range: &ITfRange) {
    unsafe {
        if let Ok(prop) = ctx.GetProperty(&GUID_PROP_ATTRIBUTE) {
            let _ = prop.Clear(ec, range);
        }
    }
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Ref<ITfThreadMgr>, tid: u32) -> Result<()> {
        self.ActivateEx(ptim, tid, 0)
    }

    fn Deactivate(&self) -> Result<()> {
        guard(|| {
            dlog!("Deactivate");
            // Commit anything still being composed.
            let ctx = self.composition_ctx.borrow().clone();
            if let Some(ctx) = ctx {
                let rest = self.composer.borrow_mut().flush();
                self.pending.borrow_mut().push_str(&rest);
                // Our client id is about to become invalid: prefer a synchronous commit.
                let this = self.to_object();
                if !request_update_with(&this, &ctx, TF_ES_SYNC | TF_ES_READWRITE) {
                    request_update(&this, &ctx);
                }
            }
            self.composer.borrow_mut().reset();

            let tm = self.thread_mgr.borrow_mut().take();
            if let Some(tm) = tm {
                unsafe {
                    if let Ok(ksm) = tm.cast::<ITfKeystrokeMgr>() {
                        let _ = ksm.UnadviseKeyEventSink(self.client_id.get());
                    }
                    let cookie = self.openclose_cookie.replace(TF_INVALID_COOKIE);
                    if cookie != TF_INVALID_COOKIE {
                        if let Ok(src) = openclose_compartment(&tm).and_then(|c| c.cast::<ITfSource>()) {
                            let _ = src.UnadviseSink(cookie);
                        }
                    }
                }
            }
            self.client_id.set(0);
            Ok(())
        })
    }
}

impl ITfTextInputProcessorEx_Impl for TextService_Impl {
    fn ActivateEx(&self, ptim: Ref<ITfThreadMgr>, tid: u32, _flags: u32) -> Result<()> {
        guard(|| unsafe {
            dlog!("ActivateEx tid={tid}");
            let tm = ptim.ok()?.clone();
            *self.thread_mgr.borrow_mut() = Some(tm.clone());
            self.client_id.set(tid);

            let ksm: ITfKeystrokeMgr = tm.cast()?;
            let sink: ITfKeyEventSink = self.to_interface();
            ksm.AdviseKeyEventSink(tid, &sink, true)?;

            let _ = ensure_open(&tm, tid);
            if let Ok(src) = openclose_compartment(&tm).and_then(|c| c.cast::<ITfSource>()) {
                let unk: IUnknown = self.to_interface();
                if let Ok(cookie) = src.AdviseSink(&ITfCompartmentEventSink::IID, &unk) {
                    self.openclose_cookie.set(cookie);
                }
            }

            // Display attribute for the composing text. Optional.
            let cat: Result<ITfCategoryMgr> = CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER);
            if let Ok(cat) = cat {
                if let Ok(atom) = cat.RegisterGUID(&GUID_DISPLAY_ATTR_INPUT) {
                    self.attr_atom.set(atom);
                }
            }
            Ok(())
        })
    }
}

impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
        dlog!("OnSetFocus {}", fforeground.as_bool());
        Ok(())
    }

    fn OnTestKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        guard(|| {
            let a = self.classify(wparam.0 as u16);
            dlog!("OnTestKeyDown vk={:#x} -> {a:?}", wparam.0);
            if a == Action::CommitAndPass {
                // A key we do not handle arrives while composing (space, Enter,
                // punctuation, arrows, shortcuts...). Once a key is reported as
                // eaten the application never sees it, so commit right now,
                // synchronously, and let the key through.
                if let Ok(ctx) = pic.ok() {
                    let rest = self.composer.borrow_mut().flush();
                    self.pending.borrow_mut().push_str(&rest);
                    let this = self.to_object();
                    if !request_update_with(&this, ctx, TF_ES_SYNC | TF_ES_READWRITE) {
                        dlog!("sync commit refused; falling back to async");
                        request_update(&this, ctx);
                    }
                }
                return Ok(FALSE);
            }
            Ok((a != Action::Pass).into())
        })
    }

    fn OnKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        guard(|| {
            let action = self.classify(wparam.0 as u16);
            dlog!("OnKeyDown vk={:#x} action={action:?} composer={:?}", wparam.0, self.composer.borrow().preedit());
            let ctx = pic.ok().ok().cloned();
            let this = self.to_object();
            let update = |ctx: &Option<ITfContext>| match ctx {
                Some(c) => request_update(&this, c),
                None => false,
            };
            match action {
                Action::Pass => Ok(FALSE),
                Action::Jamo(j) => {
                    let out = self.composer.borrow_mut().input(j);
                    self.pending.borrow_mut().push_str(&out);
                    if update(&ctx) {
                        Ok(TRUE)
                    } else {
                        // Document refused the edit (read-only, etc.): give up quietly.
                        self.composer.borrow_mut().reset();
                        self.pending.borrow_mut().clear();
                        Ok(FALSE)
                    }
                }
                Action::Backspace => {
                    self.composer.borrow_mut().backspace();
                    update(&ctx);
                    Ok(TRUE)
                }
                Action::CommitAndPass => {
                    let rest = self.composer.borrow_mut().flush();
                    self.pending.borrow_mut().push_str(&rest);
                    update(&ctx);
                    Ok(FALSE)
                }
            }
        })
    }

    fn OnTestKeyUp(&self, _pic: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn OnKeyUp(&self, _pic: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn OnPreservedKey(&self, _pic: Ref<ITfContext>, _rguid: *const GUID) -> Result<BOOL> {
        Ok(FALSE)
    }
}

impl ITfCompositionSink_Impl for TextService_Impl {
    /// The application ended our composition (focus change, mouse click...).
    /// The text stays in the document; just forget our state.
    fn OnCompositionTerminated(&self, ecwrite: u32, pcomposition: Ref<ITfComposition>) -> Result<()> {
        guard(|| {
            dlog!("OnCompositionTerminated");
            let ctx = self.composition_ctx.borrow_mut().take();
            if let (Some(ctx), Ok(comp)) = (ctx, pcomposition.ok()) {
                if let Ok(range) = unsafe { comp.GetRange() } {
                    clear_attr(ecwrite, &ctx, &range);
                }
            }
            *self.composition.borrow_mut() = None;
            self.composer.borrow_mut().reset();
            Ok(())
        })
    }
}

impl ITfCompartmentEventSink_Impl for TextService_Impl {
    /// Something (e.g. the 한/영 key via the Korean keyboard layout) closed the
    /// keyboard: reopen it, since this IME is Hangul-only.
    fn OnChange(&self, rguid: *const GUID) -> Result<()> {
        guard(|| {
            if rguid.is_null() || unsafe { *rguid } != GUID_COMPARTMENT_KEYBOARD_OPENCLOSE {
                return Ok(());
            }
            let tm = self.thread_mgr.borrow().clone();
            if let Some(tm) = tm {
                dlog!("open/close compartment changed; reopening");
                let _ = ensure_open(&tm, self.client_id.get());
            }
            Ok(())
        })
    }
}

impl ITfDisplayAttributeProvider_Impl for TextService_Impl {
    fn EnumDisplayAttributeInfo(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(EnumDisplayAttributeInfo::create(0))
    }

    fn GetDisplayAttributeInfo(&self, guid: *const GUID) -> Result<ITfDisplayAttributeInfo> {
        if guid.is_null() {
            return Err(E_INVALIDARG.into());
        }
        if unsafe { *guid } == GUID_DISPLAY_ATTR_INPUT {
            Ok(DisplayAttributeInfo.into())
        } else {
            Err(E_INVALIDARG.into())
        }
    }
}
