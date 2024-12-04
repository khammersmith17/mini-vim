use std::time::Instant;

pub struct Help {
    pub render_help: bool,
    pub time_began: Instant,
}

impl Default for Help {
    fn default() -> Self {
        Self {
            render_help: false,
            time_began: Instant::now(),
        }
    }
}
