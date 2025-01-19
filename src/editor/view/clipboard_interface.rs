use clipboard::{ClipboardContext, ClipboardProvider};
use std::error::Error;

pub struct ClipboardUtils;

impl ClipboardUtils {
    pub fn copy_text_to_clipboard(content: String) -> Result<(), Box<dyn Error>> {
        let mut ctx = ClipboardContext::new()?;
        let res = ctx.set_contents(content)?;
        Ok(res)
    }

    pub fn get_text_from_clipboard() -> Result<String, Box<dyn Error>> {
        let mut ctx = ClipboardContext::new()?;
        let res = ctx.get_contents()?;
        Ok(res)
    }
}
