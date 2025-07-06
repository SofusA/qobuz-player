use crate::{AppState, components::button_class, html, page::UnauthorizedPage, view::render};
use axum::{
    Form, Router,
    body::Body,
    extract::{Request, State},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use leptos::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth", get(index))
        .route("/auth/login", post(login))
}

pub(crate) async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    request: Request,
    next: axum::middleware::Next,
) -> (CookieJar, Response<Body>) {
    let Some(state_secret) = state.secret.clone() else {
        return (jar, next.run(request).await);
    };

    let redirect_response = (
        StatusCode::FOUND,
        [
            (
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
            ),
            (
                axum::http::header::LOCATION,
                axum::http::HeaderValue::from_static("/auth"),
            ),
        ],
    )
        .into_response();

    let secret = jar.get("secret");

    match secret {
        Some(result_secret) => {
            if state_secret != result_secret.value() {
                return (jar, redirect_response);
            }
        }
        None => return (jar, redirect_response),
    }

    (set_auth_cookie(jar, state_secret), next.run(request).await)
}

fn set_auth_cookie(jar: CookieJar, secret: String) -> CookieJar {
    let mut cookie = Cookie::new("secret", secret);
    cookie.set_same_site(SameSite::Strict);
    cookie.set_path("/");
    cookie.set_max_age(time::Duration::weeks(1));
    jar.add(cookie)
}

async fn index() -> impl IntoResponse {
    render(html! {
        <UnauthorizedPage>
            <div class="flex justify-center items-center w-full h-full">
                <form class="flex flex-col gap-4" action="/auth/login" method="post">
                    <input
                        class="p-2 w-full text-black bg-white rounded"
                        type="password"
                        id="secret"
                        name="secret"
                        placeholder="Secret"
                    />
                    <button class=button_class() type="submit">
                        Submit
                    </button>
                </form>
            </div>
        </UnauthorizedPage>
    })
}

#[derive(Deserialize)]
struct LoginParameters {
    secret: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(parameters): Form<LoginParameters>,
) -> (CookieJar, Response<Body>) {
    let response = (
        StatusCode::FOUND,
        [
            (
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
            ),
            (
                axum::http::header::LOCATION,
                axum::http::HeaderValue::from_static("/"),
            ),
        ],
        "Success",
    )
        .into_response();

    match state.secret.clone() {
        None => return (jar, response),
        Some(secret) => {
            if secret == parameters.secret {
                return (set_auth_cookie(jar, secret), response);
            };
        }
    }

    let response = (StatusCode::UNAUTHORIZED, "Bad credentials").into_response();
    (jar, response)
}
