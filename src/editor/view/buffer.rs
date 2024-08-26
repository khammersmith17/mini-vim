pub struct Buffer {
    pub text: Vec<String>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            text: vec!["Hello World".to_string()],
        }
    }
}
