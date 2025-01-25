use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_image::{provide_image_context, Image};
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    provide_image_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/start-axum.css" />
        <Title text="Welcome to Leptos" />
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors /> }.into_view()
        }>
            <main>
                <Routes>
                    <Route
                        path=""
                        view=|| {
                            view! {
                                <div
                                    style:display="flex"
                                    style:width="40rem"
                                    style:justify-content="space-between"
                                    style:margin-left="auto"
                                    style:margin-right="auto"
                                >
                                    <div>
                                        <a href="/">"Home"</a>
                                    </div>
                                    <div>
                                        <a href="/lg">"Large"</a>
                                    </div>
                                    <div>
                                        <a href="/md">"Medium"</a>
                                    </div>
                                    <div>
                                        <a href="/sm">"Small"</a>
                                    </div>
                                    <div>
                                        <a href="/no-blur">"No Blur"</a>
                                    </div>
                                </div>
                                <Outlet />
                            }
                        }
                    >

                        <Route
                            path="/"
                            view=|| {
                                view! { <h1>"Welcome to Leptos Image"</h1> }
                            }
                        />

                        <Route
                            path="/lg"
                            view=|| {
                                view! { <ImageComparison width=1000 height=1000 blur=true /> }
                            }
                        />

                        <Route
                            path="/md"
                            view=|| {
                                view! { <ImageComparison width=500 height=500 blur=true /> }
                            }
                        />

                        <Route
                            path="/sm"
                            view=|| {
                                view! { <ImageComparison width=100 height=100 blur=true /> }
                            }
                        />

                        <Route
                            path="/no-blur"
                            view=|| {
                                view! { <ImageComparison width=1000 height=1000 blur=false /> }
                            }
                        />

                    </Route>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn ImageComparison(width: u32, height: u32, blur: bool) -> impl IntoView {
    view! {
        <div
            style:margin-left="auto"
            style:margin-right="auto"
            style:display="flex"
            style:justify-content="space-around"
            style:align-items="center"
            style:gap="1rem"
        >
            <div>
                <div>
                    <h1>{format!("Optimized ({width} x {height}) with blur preview")}</h1>
                </div>
                <Image src="/cute_ferris.png" width height quality=85 blur class="test-image" />
            </div>
            <div>
                <div>
                    <h1>"Normal Image"</h1>
                </div>
                <img src="/cute_ferris.png" class="test-image" />
            </div>
        </div>
    }
}
