use leptos::logging;
use crate::optimizer::*;

use leptos::prelude::*;
use leptos_meta::Link;
use base64::{engine::general_purpose, Engine as _};

/**
 * Renders an optimized static image with optional blur placeholder and preload.
 *
 * The width/height properties ensure the layout space is reserved from the start,
 * preventing content shift when the image or placeholder loads.
 */
#[component]
pub fn Image(
    /// Image source. Should be path relative to root.
    #[prop(into)]
    src: String,
    /// Resize image height (final image), maintains aspect ratio relative to `width`.
    height: u32,
    /// Resize image width (final image), maintains aspect ratio relative to `height`.
    width: u32,
    /// Image quality (0-100).
    #[prop(default = 75_u8)]
    quality: u8,
    /// Whether to add a blur placeholder before the real image loads.
    #[prop(default = true)]
    blur: bool,
    /// Whether to add a preload <link> for this image.
    #[prop(default = false)]
    priority: bool,
    /// Lazy-load the final image.
    #[prop(default = true)]
    lazy: bool,
    /// Image alt text.
    #[prop(into, optional)]
    alt: String,
    /// Additional CSS classes for the image.
    #[prop(into, optional)]
    class: MaybeProp<String>,
) -> impl IntoView {
    // If remote (http/https), skip optimization and just return a plain <img>.
    if src.starts_with("http") {
        logging::debug_warn!("Image component only supports static images.");
        let loading = if lazy { "lazy" } else { "eager" };
        return view! {
            <img
                src=src
                alt=alt
                class=class.get()
                width=width
                height=height
                decoding="async"
                loading=loading
            />
        }
            .into_any();
    }

    // Prepare the cache descriptors for blur version and optimized version
    let blur_image = StoredValue::new(CachedImage {
        src: src.clone(),
        option: CachedImageOption::Blur(Blur {
            width: 20,
            height: 20,
            svg_width: 100,
            svg_height: 100,
            sigma: 15,
        }),
    });

    let opt_image = StoredValue::new(CachedImage {
        src: src.clone(),
        option: CachedImageOption::Resize(Resize {
            quality,
            width,
            height,
        }),
    });

    // We fetch the global image cache resource
    let resource = crate::use_image_cache_resource();
    let alt = StoredValue::new(alt);

    return view! {
        <Suspense fallback=move || {
            view! {
                // If you prefer, you could do a placeholder gray box, spinner, etc.
                <div style=move || {
                    format!("width: {}px; height: {}px; background-color: #f0f0f0;", width, height)
                } />
            }
        }>
            // Once the resource is ready, we show the real or blurred image
            {move || {
                resource
                    .get()
                    .map(|config| {
                        let images = &config.cache;
                        let handler_path = &config.api_handler_path;
                        let opt_image_url = opt_image.get_value().get_url_encoded(handler_path);
                        if blur {
                            let placeholder_svg = images
                                .iter()
                                .find(|(c, _)| blur_image.with_value(|b| b == c))
                                .map(|(_, svg_data)| svg_data.clone());
                            let svg = if let Some(svg_data) = placeholder_svg {
                                SvgImage::InMemory(svg_data)
                            } else {
                                SvgImage::Request(
                                    blur_image.get_value().get_url_encoded(handler_path),
                                )
                            };
                            return view! {
                                // Try to fetch an existing cached placeholder

                                <CacheImage
                                    svg=svg
                                    opt_image=opt_image_url
                                    alt=alt.get_value()
                                    class=class
                                    priority=priority
                                    lazy=lazy
                                    width=width
                                    height=height
                                />
                            }
                                .into_any();
                        } else {
                            let loading = if lazy { "lazy" } else { "eager" };
                            return view! {
                                // Try to fetch an existing cached placeholder

                                // Try to fetch an existing cached placeholder

                                // No blur => standard <img> with known w/h
                                <img
                                    src=opt_image_url
                                    alt=alt.get_value()
                                    class=move || class.get()
                                    width=width
                                    height=height
                                    decoding="async"
                                    loading=loading
                                />
                            }
                                .into_any();
                        }
                    })
            }}
        </Suspense>
    }.into_any()
}

enum SvgImage {
    InMemory(String),
    Request(String),
}

/// Internal component that displays the blurred placeholder (SVG)
/// in the background of the <img> until the real image is displayed.
#[component]
fn CacheImage(
    svg: SvgImage,
    #[prop(into)]
    opt_image: String,
    #[prop(into, optional)]
    alt: String,
    #[prop(into, optional)]
    class: MaybeProp<String>,
    priority: bool,
    lazy: bool,
    // Passed down to maintain the final layout from the start
    width: u32,
    height: u32,
) -> impl IntoView {
    // Construct background SVG or request URL
    let background_image = match svg {
        SvgImage::InMemory(svg_data) => {
            let svg_encoded = general_purpose::STANDARD.encode(svg_data.as_bytes());
            format!("url('data:image/svg+xml;base64,{svg_encoded}')")
        }
        SvgImage::Request(svg_url) => format!("url('{svg_url}')"),
    };

    let style = format!(
        "color: transparent;\
         background-size: cover;\
         background-position: 50% 50%;\
         background-repeat: no-repeat;\
         background-image: {background_image};"
    );

    let loading = if lazy { "lazy" } else { "eager" };

    view! {
        {if priority {
            view! { <Link rel="preload" as_="image" href=opt_image.clone() /> }.into_any()
        } else {
            ().into_any()
        }}

        // Reserve the space with width/height, apply the blur background
        <img
            src=opt_image
            alt=alt.clone()
            class=move || class.get()
            decoding="async"
            loading=loading
            width=width
            height=height
            style=style
        />
    }
}
