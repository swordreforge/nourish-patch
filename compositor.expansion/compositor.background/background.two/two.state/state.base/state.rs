use compositor_background_two_draw_element::element::ParallaxBackground;

pub struct Two {
    pub instance: Option<ParallaxBackground>,
}

impl Two {
    pub fn new() -> Self {
        return Self { instance: None };
    }
}
