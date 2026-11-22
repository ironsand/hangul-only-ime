//! Display attribute (underline) for the syllable being composed.

use std::cell::Cell;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::TextServices::*;

use crate::GUID_DISPLAY_ATTR_INPUT;

#[implement(ITfDisplayAttributeInfo)]
pub struct DisplayAttributeInfo;

impl ITfDisplayAttributeInfo_Impl for DisplayAttributeInfo_Impl {
    fn GetGUID(&self) -> Result<GUID> {
        Ok(GUID_DISPLAY_ATTR_INPUT)
    }

    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("Hangul IME composing"))
    }

    fn GetAttributeInfo(&self, pda: *mut TF_DISPLAYATTRIBUTE) -> Result<()> {
        if pda.is_null() {
            return Err(E_INVALIDARG.into());
        }
        unsafe {
            *pda = TF_DISPLAYATTRIBUTE {
                lsStyle: TF_LS_SOLID,
                fBoldLine: FALSE,
                bAttr: TF_ATTR_INPUT,
                ..Default::default()
            };
        }
        Ok(())
    }

    fn SetAttributeInfo(&self, _pda: *const TF_DISPLAYATTRIBUTE) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Reset(&self) -> Result<()> {
        Ok(())
    }
}

#[implement(IEnumTfDisplayAttributeInfo)]
pub struct EnumDisplayAttributeInfo {
    pos: Cell<u32>,
}

const COUNT: u32 = 1;

impl EnumDisplayAttributeInfo {
    pub fn create(pos: u32) -> IEnumTfDisplayAttributeInfo {
        EnumDisplayAttributeInfo { pos: Cell::new(pos) }.into()
    }
}

impl IEnumTfDisplayAttributeInfo_Impl for EnumDisplayAttributeInfo_Impl {
    fn Clone(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(EnumDisplayAttributeInfo::create(self.pos.get()))
    }

    fn Next(&self, ulcount: u32, rginfo: *mut Option<ITfDisplayAttributeInfo>, pcfetched: *mut u32) -> Result<()> {
        if rginfo.is_null() {
            return Err(E_INVALIDARG.into());
        }
        let mut fetched = 0;
        while fetched < ulcount && self.pos.get() < COUNT {
            unsafe { rginfo.add(fetched as usize).write(Some(DisplayAttributeInfo.into())) };
            fetched += 1;
            self.pos.set(self.pos.get() + 1);
        }
        if !pcfetched.is_null() {
            unsafe { *pcfetched = fetched };
        }
        if fetched == ulcount {
            Ok(())
        } else {
            Err(S_FALSE.into())
        }
    }

    fn Reset(&self) -> Result<()> {
        self.pos.set(0);
        Ok(())
    }

    fn Skip(&self, ulcount: u32) -> Result<()> {
        let new = self.pos.get().saturating_add(ulcount);
        self.pos.set(new.min(COUNT));
        if new <= COUNT {
            Ok(())
        } else {
            Err(S_FALSE.into())
        }
    }
}
