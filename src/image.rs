use crate::optimizer::*;

use leptos::*;
use leptos_meta::Link;

/**
 */

/// Image component for rendering optimized static images.
/// Images MUST be static. Will not work with dynamic images.
#[component]
pub fn Image<'a>(
    /// Image source. Should be path relative to root.
    #[prop(into)]
    src: String,
    /// Resize image height.
    height: u32,
    /// Resize image width.
    width: u32,
    /// Image quality. 0-100.
    #[prop(default = 75_u8)]
    quality: u8,
    /// Filter type for the conversion : Nearest, Triangle, CatmullRom, Gaussian, Lanczos3
    #[prop(default = "catmullrom")]
    filter: &'a str,
    /// Resize type for the conversion : Fit, Cover, Thumbnail
    #[prop(default = "fit")]
    resize_type: &'a str,
    /// Will add blur image to head if true.
    #[prop(default = false)]
    blur: bool,
    /// Will add preload link to head if true.
    #[prop(default = false)]
    priority: bool,
    /// Lazy load image.
    #[prop(default = true)]
    lazy: bool,
    /// Image alt text.
    #[prop(into, optional)]
    alt: String,
    /// Style class for image.
    #[prop(into, optional)]
    class: Option<AttributeValue>,
) -> impl IntoView {
    if src.starts_with("http") {
        logging::debug_warn!("Image component only supports static images.");
        let loading = if lazy { "lazy" } else { "eager" };
        return view! { <img src=src alt=alt class=class loading=loading/> }.into_view();
    }

    let blur_image = {
        CachedImage {
            src: src.clone(),
            option: CachedImageOption::Blur(Blur {
                width: 20,
                height: 20,
                svg_width: 100,
                svg_height: 100,
                sigma: 15,
            }),
        }
    };

    let opt_image = {
        CachedImage {
            src: src.clone(),
            option: CachedImageOption::Resize(Resize {
                quality,
                filter: filter.parse().unwrap_or_default(),
                width,
                height,
                resize_type: resize_type.parse().unwrap_or_default(),
            }),
        }
    };

    // Retrieve value from Cache if it exists. Doing this per-image to allow image introspection.
    let resource = crate::use_image_cache_resource();

    let blur_image = store_value(blur_image);
    let opt_image = store_value(opt_image);
    let alt = store_value(alt);
    let class = store_value(class.map(|c| c.into_attribute_boxed()));

    view! {
        <Suspense fallback=|| ()>
            {move || {
                resource
                    .get()
                    .map(|config| {
                        let images = config.cache;
                        let handler_path = config.api_handler_path;
                        let opt_image = opt_image.get_value().get_url_encoded(&handler_path);
                        if blur {
                            let placeholder_svg = images
                                .iter()
                                .find(|(c, _)| blur_image.with_value(|b| b == c))
                                .map(|c| c.1.clone());
                            let svg = {
                                if let Some(svg_data) = placeholder_svg {
                                    SvgImage::InMemory(svg_data)
                                } else {
                                    SvgImage::Request(
                                        blur_image.get_value().get_url_encoded(&handler_path),
                                    )
                                }
                            };
                            let class = class.get_value();
                            let alt = alt.get_value();
                            view! { <CacheImage lazy svg opt_image alt class=class priority/> }
                                .into_view()
                        } else {
                            let loading = if lazy { "lazy" } else { "eager" };
                            view! {
                                <img
                                    alt=alt.get_value()
                                    class=class.get_value()
                                    decoding="async"
                                    loading=loading
                                    src=opt_image
                                    width=width
                                    height=height
                                />
                            }
                                .into_view()
                        }
                    })
            }}

        </Suspense>
    }
}

enum SvgImage {
    InMemory(String),
    Request(String),
}

#[component]
fn CacheImage(
    svg: SvgImage,
    #[prop(into)] opt_image: String,
    #[prop(into, optional)] alt: String,
    class: Option<Attribute>,
    priority: bool,
    lazy: bool,
) -> impl IntoView {
    use base64::{engine::general_purpose, Engine as _};

    let style = {
        let background_image = match svg {
            SvgImage::InMemory(svg_data) => {
                let svg_encoded = general_purpose::STANDARD.encode(svg_data.as_bytes());
                format!("url('data:image/svg+xml;base64,{svg_encoded}')")
            }
            SvgImage::Request(svg_url) => {
                format!("url('{}')", svg_url)
            }
        };
        let style = format!(
            "color:transparent;background-size:cover;background-position:50% 50%;background-repeat:no-repeat;background-image:{background_image};",
        );

        style
    };

    let loading = if lazy { "lazy" } else { "eager" };

    view! {
        {if priority {
            view! { <Link rel="preload" as_="image" href=opt_image.clone()/> }.into_view()
        } else {
            ().into_view()
        }}

        <img
            alt=alt.clone()
            class=class
            decoding="async"
            loading=loading
            src=opt_image
            style=style
        />
    }
}




pub struct Ruleset{
    pub width: u32,
    pub height: u32,
    pub quality: u8,
    pub filter: String,
    pub resize_type: String,
}
/// Picture component for rendering optimized static images.
/// Images MUST be static. Will not work with dynamic images.
/// Will resize an image based on rules and dimensions.
#[component]
pub fn Picture(
    /// Image source. Should be path relative to root.
    #[prop(into)] ///
    src: String, ///
    /// A rule that based on screen width and height will return a Resize struct.
    ruleset: fn(usize, usize) -> Ruleset,
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



    view! {

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
            resize_type=&rules.resize_type
            filter=&rules.filter
        />
    }
}
