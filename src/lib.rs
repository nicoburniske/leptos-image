#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! # Leptos Image
//!
//! > Crafted with inspiration from Next.js
//!
//! Images make a substantial impact on the size and performance of a website, so why not get them right?
//!
//! Enter Leptos `<Image/>`, a component that enhances the standard HTML `<img>` element with automatic image optimization features.
//!
//! ## Features
//!
//! - **Size Optimization**: Automatically resizes images and converts them to the modern `.webp` format for an ideal balance of size and quality.
//! - **Low-Quality Image Placeholders (LQIP)**: Embeds SVG placeholders extracted from original images directly into your server-side rendered HTML, improving perceived performance by displaying content while the full-quality image loads.
//! - **Faster Page Load**: Prioritizes key images that impact the Largest Contentful Paint (LCP) with the `priority` prop, injecting a preload `<link>` into the document head to accelerate load times.
//!
//! ## Getting Started
//!
//! The crate focuses on creating optimized images for static content in Leptos projects, a full-stack web framework in Rust.
//!
//! ### Setup Process
//!
//! 1. **Provide Image Context**: Initialize your Leptos application with `leptos_image::provide_image_context` to grant it read access to the image cache.
//!    ```
//!    use leptos::*;
//!
//!    #[component]
//!    fn App() -> impl IntoView {
//!        leptos_image::provide_image_context();
//!        // Your app content here
//!    }
//!    ```
//! 2. **Integrate with Leptos Routes**: Ensure your router includes the `ImageOptimizer` context when setting up Leptos routes.
//! 3. **Axum State Configuration**: Incorporate `ImageOptimizer` into your app's Axum state for centralized management.
//! 4. **Cache Route Configuration**: Add a dedicated route to your router for serving optimized images from the cache.
//!
//! ### Example Implementation
//!
//! Here’s how you can integrate the Image Optimizer into your Leptos application:
//!
//! ```
//!
//! # use leptos_image::*;
//! # use leptos::*;
//! # use axum::*;
//! # use axum::routing::post;
//! # use leptos_axum::{generate_route_list, handle_server_fns, LeptosRoutes};
//!
//! #[cfg(feature = "ssr")]
//! async fn your_main_function() {
//!     let options = get_configuration(None).await.unwrap().leptos_options;
//!     let optimizer = ImageOptimizer::new("/__cache/image", options.site_root.clone(), 1);
//!     let state = AppState { leptos_options: options, optimizer: optimizer.clone() };
//!
//!     let router: Router<()> = Router::new()
//!         .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
//!         // Adding cache route
//!         .image_cache_route(&state)
//!         // Provide the optimizer to Leptos context
//!         .leptos_routes_with_context(&state, generate_route_list(App), optimizer.provide_context(), App)
//!         .with_state(state);
//!
//!     // Rest of your function...
//! }
//!
//! // Composite App State with the optimizer and Leptos options.
//! #[derive(Clone, axum::extract::FromRef)]
//! struct AppState {
//!     leptos_options: LeptosOptions,
//!     optimizer: ImageOptimizer,
//! }
//!
//! #[component]
//! fn App() -> impl IntoView {
//!     provide_image_context();
//!     // Your app content here
//! }
//! ```
//!
//! This setup ensures your Leptos application is fully equipped to deliver optimized images, enhancing the performance and user experience of your web projects.
//!
//! Now you can use the Image Component anywhere in your app!

//! ```
//! use leptos::*;
//! use leptos_image::*;
//!
//! #[component]
//! pub fn MyImage() -> impl IntoView {
//!     view! {
//!         <Image
//!             src="/cute_ferris.png"
//!             blur=true
//!             width=750
//!             height=500
//!             quality=85
//!         />
//!     }
//! }
//! ```
//!

mod image;
mod optimizer;
mod provider;
#[cfg(feature = "ssr")]
mod routes;
mod picture;

pub use image::*;
pub use optimizer::{Filter, Resize, ResizeType};
#[cfg(feature = "ssr")]
pub use optimizer::ImageOptimizer;
pub use provider::*;
#[cfg(feature = "ssr")]
pub use routes::*;
