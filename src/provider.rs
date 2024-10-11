use crate::optimizer::CachedImage;
use leptos::*;

/// Provides Image Cache Context so that Images can use their blur placeholders if they exist.
///
/// This should go in the base of your Leptos <App/>.
///
/// Example
///
/// ```
/// use leptos::*;
///
/// #[component]
/// pub fn App() -> impl IntoView {
///     leptos_image::provide_image_context();
///
///     view!{
///       <div/>
///     }
/// }
///
/// ```
pub fn provide_image_context() {
    let resource: ImageResource = create_blocking_resource(
        || (),
        |_| async {
            get_image_config()
                .await
                .expect("Failed to retrieve image cache")
        },
    );

    provide_context(resource);
}

type ImageResource = Resource<(), ImageConfig>;

#[doc(hidden)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageConfig {
    pub(crate) api_handler_path: String,
    pub(crate) cache: Vec<(CachedImage, String)>,
}

pub(crate) fn use_image_cache_resource() -> ImageResource {
    use_context::<ImageResource>().expect("Missing Image Resource")
}

#[server(GetImageCache)]
pub(crate) async fn get_image_config() -> Result<ImageConfig, ServerFnError> {
    let optimizer = use_optimizer()?;

    let cache = optimizer
        .cache
        .iter()
        .map(|entry| (entry.key().clone(), entry.value().clone()))
        .collect();

    let api_handler_path = optimizer.api_handler_path.clone();

    Ok(ImageConfig {
        api_handler_path,
        cache,
    })
}

#[cfg(feature = "ssr")]
pub(crate) fn use_optimizer() -> Result<crate::ImageOptimizer, ServerFnError> {
    use_context::<crate::ImageOptimizer>()
        .ok_or_else(|| ServerFnError::ServerError("Image Optimizer Missing.".into()))
}
