use crate::optimizer::{CachedImage, CachedImageOption, CreateImageError, ImageOptimizer};
use axum::extract::FromRef;
use axum::response::Response as AxumResponse;
use axum::{
    body::Body,
    http::{Request, Response, Uri},
    response::IntoResponse,
};
use leptos::LeptosOptions;
use std::convert::Infallible;
use tower::ServiceExt;
use tower_http::services::fs::ServeFileSystemResponseBody;
use tower_http::services::ServeDir;

/// This trait prevents using incorrect route for image cache handler.
pub trait ImageCacheRoute<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Adds a route to the app for serving cached images.
    /// Requires an axum State that contains the optimizer [`crate::ImageOptimizer`].
    ///
    /// ```
    /// use leptos_image::*;
    /// use leptos::*;
    /// use axum::*;
    /// use axum::routing::post;
    /// use leptos_axum::{generate_route_list, handle_server_fns, LeptosRoutes};
    ///
    /// #[cfg(feature = "ssr")]
    /// async fn your_main_function() {
    ///
    ///   let options = get_configuration(None).await.unwrap().leptos_options;
    ///   let optimizer = ImageOptimizer::new(options.site_root.clone(), 1);
    ///   let state = AppState {leptos_options: options, optimizer: optimizer.clone() };
    ///   let routes = generate_route_list(App);
    ///
    ///   let router: Router<()> = Router::new()
    ///    .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
    ///    // Add a handler for serving the cached images.
    ///    .image_cache_route(&state)
    ///    .leptos_routes_with_context(&state, routes, optimizer.provide_context(), App)
    ///    .with_state(state);
    ///
    ///   // Rest of your function ...
    /// }
    ///
    /// // Composite App State with the optimizer and leptos options.
    /// #[derive(Clone, axum::extract::FromRef)]
    /// struct AppState {
    ///   leptos_options: leptos::LeptosOptions,
    ///   optimizer: leptos_image::ImageOptimizer,
    /// }
    ///
    /// #[component]
    /// fn App() -> impl IntoView {
    ///   provide_image_context();
    ///   ()
    /// }
    ///
    /// ```
    ///
    ///
    fn image_cache_route(self, state: &S) -> Self;
}

impl<S> ImageCacheRoute<S> for axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    LeptosOptions: FromRef<S>,
    ImageOptimizer: FromRef<S>,
{
    fn image_cache_route(self, state: &S) -> Self {
        let options = leptos::LeptosOptions::from_ref(state);
        let optimizer = ImageOptimizer::from_ref(state);

        let path = optimizer.api_handler_path.clone();
        let handler = move |req: Request<Body>| image_cache_handler_inner(options, optimizer, req);

        self.route(&path, axum::routing::get(handler))
    }
}

async fn image_cache_handler_inner(
    options: LeptosOptions,
    optimizer: ImageOptimizer,
    req: Request<Body>,
) -> AxumResponse {
    let root = options.site_root.clone();
    let cache_result = check_cache_image(&optimizer, req.uri().clone()).await;

    match cache_result {
        Ok(Some(uri)) => {
            let response = execute_file_handler(uri, &root).await.unwrap();
            response.into_response()
        }

        Ok(None) => Response::builder()
            .status(404)
            .body("Invalid Image.".to_string())
            .unwrap()
            .into_response(),

        Err(e) => {
            tracing::error!("Failed to create image: {:?}", e);
            Response::builder()
                .status(500)
                .body("Error creating image".to_string())
                .unwrap()
                .into_response()
        }
    }
}

async fn execute_file_handler(
    uri: Uri,
    root: &str,
) -> Result<Response<ServeFileSystemResponseBody>, Infallible> {
    let req = Request::builder()
        .uri(uri.clone())
        .body(Body::empty())
        .unwrap();
    ServeDir::new(root).oneshot(req).await
}

async fn check_cache_image(
    optimizer: &ImageOptimizer,
    uri: Uri,
) -> Result<Option<Uri>, CreateImageError> {
    let url = uri.to_string();

    let cache_image = {
        if let Some(img) = CachedImage::from_url_encoded(&url).ok() {
            let result = optimizer.create_image(&img).await;

            if let Ok(true) = result {
                tracing::info!("Created Image: {:?}", img);
            }

            result?;

            img
        } else {
            return Ok(None);
        }
    };

    let file_path = cache_image.get_file_path();

    add_file_to_cache(optimizer, cache_image).await;

    let uri_string = "/".to_string() + &file_path;
    let maybe_uri = (uri_string).parse::<Uri>().ok();

    if let Some(uri) = maybe_uri {
        Ok(Some(uri))
    } else {
        tracing::error!("Failed to create uri: File path {file_path}");
        Ok(None)
    }
}

// When the image is created, it will be added to the cache.
// Mostly helpful for dev server startup.
async fn add_file_to_cache(optimizer: &ImageOptimizer, image: CachedImage) {
    if let CachedImageOption::Blur(_) = image.option {
        add_image_cache(optimizer, vec![image]).await;
    }
}

pub(crate) async fn add_image_cache<I>(optimizer: &crate::optimizer::ImageOptimizer, images: I)
where
    I: IntoIterator<Item = CachedImage>,
{
    let images = images
        .into_iter()
        .filter(|image| matches!(image.option, crate::optimizer::CachedImageOption::Blur(_)))
        .filter(|image| optimizer.cache.get(&image).is_none());

    for image in images {
        let path = optimizer.get_file_path_from_root(&image);
        match tokio::fs::read_to_string(path).await {
            Ok(data) => {
                optimizer.cache.insert(image, data);
                tracing::info!("Added image to cache with size {}", optimizer.cache.len())
            }
            Err(e) => {
                tracing::error!("Failed to read image: {:?} with error: {:?}", image, e);
            }
        }
    }
}
