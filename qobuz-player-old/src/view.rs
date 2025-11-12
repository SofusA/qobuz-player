use axum::response::IntoResponse;
use leptos::prelude::*;

#[macro_export]
macro_rules! html {
    ($($body:tt)*) => {
        leptos::view!($($body)*)
    };
}

pub(crate) fn render(view: impl IntoView) -> axum::response::Response {
    to_response(view.to_html())
}

fn to_response(html: String) -> axum::response::Response {
    (
        [
            (
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
            ),
            (
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-cache"),
            ),
        ],
        html,
    )
        .into_response()
}

#[component]
pub(crate) fn lazy_load_component(url: String) -> impl IntoView {
    html! { <div hx-get=url hx-target="this" hx-trigger="load" /> }
}
