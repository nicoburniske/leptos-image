use std::fmt::Display;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

/// ImageOptimizer enables image optimization and caching.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone)]
pub struct ImageOptimizer {
    pub(crate) api_handler_path: String,
    pub(crate) root_file_path: String,
    pub(crate) semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    pub(crate) cache: std::sync::Arc<dashmap::DashMap<CachedImage, String>>,
}

#[cfg(feature = "ssr")]
impl ImageOptimizer {
    /// Creates a new ImageOptimizer.
    /// api_handler_path is the path where the image handler is located in the server router.
    /// Parallelism denotes the number of images that can be created at once.
    /// Useful to limit to prevent overloading the server.
    pub fn new(
        api_handler_path: impl Into<String>,
        root_file_path: impl Into<String>,
        parallelism: usize,
    ) -> Self {
        let semaphore = tokio::sync::Semaphore::new(parallelism);
        let semaphore = std::sync::Arc::new(semaphore);
        Self {
            api_handler_path: api_handler_path.into(),
            root_file_path: root_file_path.into(),
            semaphore,
            cache: std::sync::Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Creates a context function to provide the optimizer.
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
    ///   let optimizer = ImageOptimizer::new("/__cache/image", options.site_root.clone(), 1);
    ///   let state = AppState {leptos_options: options, optimizer: optimizer.clone() };
    ///   let routes = generate_route_list(App);
    ///
    ///   let router: Router<()> = Router::new()
    ///    .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
    ///    .image_cache_route(&state)
    ///    // Use provide_context()
    ///    .leptos_routes_with_context(&state, routes, optimizer.provide_context(), App)
    ///    .with_state(state);
    ///
    ///   // Rest of your function ...
    /// }
    ///
    /// // Composite App State with the optimizer and leptos options.
    /// #[derive(Clone, axum::extract::FromRef)]
    /// struct AppState {
    ///   leptos_options: LeptosOptions,
    ///   optimizer: ImageOptimizer,
    /// }
    ///
    /// #[component]
    /// fn App() -> impl IntoView {
    ///   ()
    /// }
    /// ```
    pub fn provide_context(&self) -> impl Fn() + 'static + Clone + Send {
        let optimizer = self.clone();
        move || {
            leptos::provide_context(optimizer.clone());
        }
    }

    pub(crate) async fn create_image(
        &self,
        cache_image: &CachedImage,
    ) -> Result<bool, CreateImageError> {
        let root = self.root_file_path.as_str();
        {
            let option = if let CachedImageOption::Resize(_) = cache_image.option {
                "Resize"
            } else {
                "Blur"
            };
            tracing::debug!("Creating {option} image for {}", &cache_image.src);
        }

        let relative_path_created = self.get_file_path(&cache_image);

        let save_path = path_from_segments(vec![root, &relative_path_created]);
        let absolute_src_path = path_from_segments(vec![root, &cache_image.src]);

        if file_exists(&save_path).await {
            Ok(false)
        } else {
            let _ = self
                .semaphore
                .acquire()
                .await
                .expect("Failed to acquire semaphore");
            let task = tokio::task::spawn_blocking({
                let option = cache_image.option.clone();
                move || create_optimized_image(option, absolute_src_path, save_path)
            });

            match task.await {
                Err(join_error) => Err(CreateImageError::JoinError(join_error)),
                Ok(Err(err)) => Err(err),
                Ok(Ok(_)) => Ok(true),
            }
        }
    }

    #[cfg(feature = "ssr")]
    pub(crate) fn get_file_path_from_root(&self, cache_image: &CachedImage) -> String {
        let path = path_from_segments(vec![
            self.root_file_path.as_ref(),
            &self.get_file_path(cache_image),
        ]);
        path.as_path().to_string_lossy().to_string()
    }

    pub(crate) fn get_file_path(&self, cache_image: &CachedImage) -> String {
        use base64::{engine::general_purpose, Engine as _};
        // I'm worried this name will become too long.
        // names are limited to 255 bytes on most filesystems.

        let encode = serde_qs::to_string(&cache_image).unwrap();
        let encode = general_purpose::STANDARD.encode(encode);

        let mut path = path_from_segments(vec!["cache/image", &encode, &cache_image.src]);

        if let CachedImageOption::Resize { .. } = cache_image.option {
            path.set_extension("webp");
        } else {
            path.set_extension("svg");
        };

        path.as_path().to_string_lossy().to_string()
    }
}

#[cfg(feature = "ssr")]
fn create_optimized_image<P>(
    config: CachedImageOption,
    source_path: P,
    save_path: P,
) -> Result<(), CreateImageError>
where
    P: AsRef<std::path::Path> + AsRef<std::ffi::OsStr>,
{
    use webp::*;

    match config {
        CachedImageOption::Resize(Resize {
            width,
            height,
            quality,
            filter,
            resize_type,
        }) => {
            let mut img = image::open(source_path)?;
            resize_type.resize(&mut img, width, height, filter);

            // Create the WebP encoder for the above image
            let encoder: Encoder = Encoder::from_image(&img).unwrap();
            // Encode the image at a specified quality 0-100
            let webp: WebPMemory = encoder.encode(quality as f32);
            create_nested_if_needed(&save_path)?;
            std::fs::write(save_path, &*webp)?;

            Ok(())
        }
        CachedImageOption::Blur(blur) => {
            let svg = create_image_blur(source_path, blur)?;
            create_nested_if_needed(&save_path)?;
            std::fs::write(save_path, &*svg)?;
            Ok(())
        }
    }
}

#[cfg(feature = "ssr")]
fn create_image_blur<P>(source_path: P, blur: Blur) -> Result<String, CreateImageError>
where
    P: AsRef<std::path::Path> + AsRef<std::ffi::OsStr>,
{
    use webp::*;

    let img = image::open(source_path).map_err(|e| CreateImageError::ImageError(e))?;

    let Blur {
        width,
        height,
        svg_height,
        svg_width,
        sigma,
    } = blur;

    let img = img.resize(width, height, image::imageops::FilterType::Nearest);

    // Create the WebP encoder for the above image
    let encoder: Encoder = Encoder::from_image(&img).unwrap();
    // Encode the image at a specified quality 0-100
    let webp: WebPMemory = encoder.encode(80.0);

    // Encode the image to base64
    use base64::{engine::general_purpose, Engine as _};
    let encoded = general_purpose::STANDARD.encode(&*webp);

    let uri = format!("data:image/webp;base64,{}", encoded);

    let svg = format!(
        r#"
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="100%" height="100%" viewBox="0 0 {svg_width} {svg_height}" preserveAspectRatio="none">
    <filter id="a" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
        <feGaussianBlur stdDeviation="{sigma}" edgeMode="duplicate"/>
        <feComponentTransfer>
            <feFuncA type="discrete" tableValues="1 1"/>
        </feComponentTransfer>
    </filter>
    <image filter="url(#a)" x="0" y="0" height="100%" width="100%" href="{uri}"/>
</svg>
"#,
    );

    Ok(svg)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Hash)]
pub struct CachedImage {
    pub(crate) src: String,
    pub(crate) option: CachedImageOption,
}

impl Display for CachedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.option {
            CachedImageOption::Resize(resize) => write!(
                f,
                "ImageResize {} ({}x{} @ {}% quality)",
                self.src, resize.width, resize.height, resize.quality,
            ),
            CachedImageOption::Blur(_) => write!(f, "ImageBlur {}", self.src),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Hash)]
pub(crate) enum CachedImageOption {
    #[serde(rename = "r")]
    Resize(Resize),
    #[serde(rename = "b")]
    Blur(Blur),
}

/// The type of resize.
/// Fit will respect the original aspect ratio.
/// Cover will first try to respect the original aspect ratio, then crop the image to fill the entire target size.
/// Thumbnail respect the original aspect ratio and is usually faster than other algorithms to downsize an image. Do not use if the resizing is very similar to orignal image, this can create artifacts.
#[derive(Clone, Debug, PartialEq, Default, Eq, Deserialize, Serialize, Hash)]
pub enum ResizeType{
    #[default]
    Fit,
    Cover,
    Thumbnail,
}

#[cfg(feature = "ssr")]
impl ResizeType{
    pub(crate) fn resize(&self, img: &mut image::DynamicImage, width: u32, height: u32, filter: Filter) {
       match self {
           ResizeType::Fit => {*img = img.resize_exact(width, height, filter.into())}
           ResizeType::Cover => {*img = img.resize_to_fill(width, height, filter.into())}
           ResizeType::Thumbnail => {*img = img.thumbnail(width, height)}
       }
    }
}

impl FromStr for ResizeType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fit" => Ok(ResizeType::Fit),
            "cover" => Ok(ResizeType::Cover),
            "thumbnail" => Ok(ResizeType::Thumbnail),
            _ => Err(()),
        }
    }
}

impl Display for ResizeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResizeType::Fit => write!(f, "fit"),
            ResizeType::Cover => write!(f, "cover"),
            ResizeType::Thumbnail => write!(f, "thumbnail"),
        }
    }
}

/// Filter type for the conversion : Nearest, Triangle, CatmullRom, Gaussian, Lanczos3
/// Default to CatmullRom
/// Lanczos3 is a solid option
#[derive(Clone, Debug, PartialEq, Default, Eq, Deserialize, Serialize, Hash)]
pub enum Filter {
    #[default]
    CatmullRom,
    Gaussian,
    Nearest,
    Triangle,
    Lanczos3,
}

#[cfg(feature = "ssr")]
impl From<Filter> for image::imageops::FilterType {
    fn from(value: Filter) -> Self {
        match value {
            Filter::CatmullRom => image::imageops::FilterType::CatmullRom,
            Filter::Gaussian => image::imageops::FilterType::Gaussian,
            Filter::Nearest => image::imageops::FilterType::Nearest,
            Filter::Triangle => image::imageops::FilterType::Triangle,
            Filter::Lanczos3 => image::imageops::FilterType::Lanczos3,
        }
    }
}

impl FromStr for Filter {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "catmullrom" => Ok(Filter::CatmullRom),
            "gaussian" => Ok(Filter::Gaussian),
            "nearest" => Ok(Filter::Nearest),
            "triangle" => Ok(Filter::Triangle),
            "lanczos3" => Ok(Filter::Lanczos3),
            _ => Err(()),
        }
    }
}

impl Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Filter::CatmullRom => write!(f, "catmullRom"),
            Filter::Gaussian => write!(f, "gaussian"),
            Filter::Nearest => write!(f, "nearest"),
            Filter::Triangle => write!(f, "triangle"),
            Filter::Lanczos3 => write!(f, "lanczos3"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Hash)]
#[serde(rename = "r")]
pub struct Resize {
    #[serde(rename = "w")]
    pub width: u32,
    #[serde(rename = "h")]
    pub height: u32,
    #[serde(rename = "q")]
    pub quality: u8,
    #[serde(rename = "f")]
    pub filter: Filter,
    #[serde(rename = "t")]
    pub resize_type: ResizeType,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Hash)]
#[serde(rename = "b")]
pub(crate) struct Blur {
    #[serde(rename = "w")]
    pub width: u32,
    #[serde(rename = "h")]
    pub height: u32,
    #[serde(rename = "sw")]
    pub svg_width: u32,
    #[serde(rename = "sh")]
    pub svg_height: u32,
    #[serde(rename = "s")]
    pub sigma: u8,
}

#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
pub enum CreateImageError {
    // Unexpected(String),
    #[error("Image Error: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("Join Error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
}

impl CachedImage {
    pub(crate) fn get_url_encoded(&self, handler_path: impl AsRef<str>) -> String {
        let params = serde_qs::to_string(&self).unwrap();
        format!("{}?{}", handler_path.as_ref(), params)
    }

    #[cfg(feature = "ssr")]
    pub(crate) fn get_file_path(&self) -> String {
        use base64::{engine::general_purpose, Engine as _};
        // I'm worried this name will become too long.
        // names are limited to 255 bytes on most filesystems.

        let encode = serde_qs::to_string(&self).unwrap();
        let encode = general_purpose::STANDARD.encode(encode);

        let mut path = path_from_segments(vec!["cache/image", &encode, &self.src]);

        if let CachedImageOption::Resize { .. } = self.option {
            path.set_extension("webp");
        } else {
            path.set_extension("svg");
        };

        path.as_path().to_string_lossy().to_string()
    }

    #[allow(dead_code)]
    #[cfg(feature = "ssr")]
    // TODO: Fix this. Super Yuck.
    pub(crate) fn from_file_path(path: &str) -> Option<Self> {
        use base64::{engine::general_purpose, Engine as _};
        path.split('/')
            .filter_map(|s| {
                general_purpose::STANDARD
                    .decode(s)
                    .ok()
                    .and_then(|s| String::from_utf8(s).ok())
            })
            .find_map(|encoded| serde_qs::from_str(&encoded).ok())
    }

    #[cfg(feature = "ssr")]
    pub(crate) fn from_url_encoded(url: &str) -> Result<CachedImage, serde_qs::Error> {
        let url = url.split('?').filter(|s| *s != "?").last().unwrap_or(url);
        let result: Result<CachedImage, serde_qs::Error> = serde_qs::from_str(url);
        result
    }
}

#[cfg(feature = "ssr")]
fn path_from_segments(segments: Vec<&str>) -> std::path::PathBuf {
    segments
        .into_iter()
        .map(|s| s.trim_start_matches('/'))
        .map(|s| s.trim_end_matches('/'))
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(feature = "ssr")]
async fn file_exists<P>(path: P) -> bool
where
    P: AsRef<std::path::Path>,
{
    tokio::fs::metadata(path).await.is_ok()
}

#[cfg(feature = "ssr")]
fn create_nested_if_needed<P>(path: P) -> std::io::Result<()>
where
    P: AsRef<std::ffi::OsStr>,
{
    match std::path::Path::new(&path).parent() {
        Some(p) if !(p).exists() => std::fs::create_dir_all(p),
        Some(_) => Ok(()),
        None => Ok(()),
    }
}

// Test module
#[cfg(test)]
mod optimizer_tests {
    use super::*;

    #[test]
    fn url_encode() {
        let img = CachedImage {
            src: "test.jpg".to_string(),
            option: CachedImageOption::Resize(Resize {
                quality: 75,
                width: 100,
                height: 100,
                filter: Filter::CatmullRom,
                resize_type: ResizeType::Fit,
            }),
        };

        let encoded = img.get_url_encoded("/cache/image/test");
        let decoded: CachedImage = CachedImage::from_url_encoded(&encoded).unwrap();

        dbg!(encoded);
        assert_eq!(img, decoded);
    }

    const TEST_IMAGE: &str = "./example/start-axum/public/cute_ferris.png";

    #[test]
    fn file_path() {
        let spec = CachedImage {
            src: TEST_IMAGE.to_string(),
            option: CachedImageOption::Blur(Blur {
                width: 25,
                height: 25,
                svg_height: 100,
                svg_width: 100,
                sigma: 20,
            }),
        };

        let file_path = spec.get_file_path();

        dbg!(spec.get_file_path());

        let result = CachedImage::from_file_path(&file_path).unwrap();

        assert_eq!(spec, result);
    }

    #[test]
    fn create_blur() {
        let result = create_image_blur(
            TEST_IMAGE.to_string(),
            Blur {
                width: 25,
                height: 25,
                svg_height: 100,
                svg_width: 100,
                sigma: 20,
            },
        );
        assert!(result.is_ok());
        println!("{}", result.unwrap());
    }

    #[test]
    fn create_and_save_blur() {
        let spec = CachedImage {
            src: TEST_IMAGE.to_string(),
            option: CachedImageOption::Blur(Blur {
                width: 25,
                height: 25,
                svg_height: 100,
                svg_width: 100,
                sigma: 20,
            }),
        };

        let file_path = spec.get_file_path();

        let result = create_optimized_image(spec.option, TEST_IMAGE.to_string(), file_path.clone());

        assert!(result.is_ok());

        println!("Saved SVG at {file_path}");
    }

    #[test]
    fn create_opt_image() {
        let spec = CachedImage {
            src: TEST_IMAGE.to_string(),
            option: CachedImageOption::Resize(Resize {
                quality: 75,
                width: 100,
                height: 100,
                filter: Filter::CatmullRom,
                resize_type: ResizeType::Fit,
            }),
        };

        let file_path = spec.get_file_path();

        let result = create_optimized_image(spec.option, TEST_IMAGE.to_string(), file_path.clone());

        assert!(result.is_ok());

        println!("Saved WebP at {file_path}");
    }
}
