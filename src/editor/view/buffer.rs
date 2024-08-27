#[derive(Default)]
pub struct Buffer {
    pub text: Vec<String>,
}

impl Buffer {
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}
