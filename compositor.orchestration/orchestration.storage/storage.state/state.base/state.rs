use std::collections::HashMap;

pub struct Storage {
    pub nested: bool,
    // pub config: Config,
}
impl Storage  {
    pub fn new(nested: bool) -> Self {
        return Self {
            nested
        }
    }
}

