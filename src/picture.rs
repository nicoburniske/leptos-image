use crate::optimizer::Resize;
use crate::Image;
use leptos::{component, view, AttributeValue, IntoView};

/// Picture component for rendering optimized static images.
/// Images MUST be static. Will not work with dynamic images.
/// Will resize an image based on rules and dimensions.
#[component]
pub fn Picture(
    /// Image source. Should be path relative to root.
    #[prop(into)]///
    src: String,///
    /// A rule that based on screen width and height will return a Resize struct.
    #[prop(into)] ruleset:
    fn(usize, usize) -> Resize,
    /// Will add blur image to head if true.
    #[prop(default = false)]
    blur: bool,
    /// Will add preload to the head if true.
    #[prop(default = false)]
    priority: bool,
    /// Will add lazy loading to the head if true.
    #[prop(default = true)]
    lazy: bool,
    /// Image alt text.
    #[prop(into, optional)]
    alt: String,
    /// Style class for image
    #[prop(into, optional)]
    class: Option<AttributeValue>,
    ) -> impl IntoView {

    let screen = leptos::window();
    let screen_width = screen.inner_width().unwrap_or_default().as_f64().unwrap_or_default() as usize;
    let screen_height = screen.inner_height().unwrap_or_default().as_f64().unwrap_or_default() as usize;

    let rules = ruleset(screen_width, screen_height);

    let resize: String = rules.resize_type.to_string();
    let filter: String = rules.filter.to_string();

    view!{

        <Image
            src=src
            alt=alt
            class=class
            priority=priority
            blur=blur
            lazy=lazy
            width=rules.width
            height=rules.height
            quality=rules.quality
            resize_type=&resize
            filter=&filter
        />
    }
}
