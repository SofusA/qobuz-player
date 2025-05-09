use std::sync::OnceLock;

use crate::{html, page::Page, view::render};
use axum::{
    Form, Router,
    response::IntoResponse,
    routing::{get, post},
};
use leptos::prelude::*;
use serde::Deserialize;

static SECRET: OnceLock<String> = OnceLock::new();

pub fn routes(secret: String) -> Router {
    SECRET.set(secret).unwrap();
    Router::new()
        .route("/auth", get(index))
        .route("/auth/login", post(login))
}

// TODO: Sæt cookie der bare er secret
// TODO: Middleware der tjekker vis cookie ikke er der eller ikke passer, redirect til auth
// TODO: Hvis forkert secret vis fejl
// TODO: Hvis god login, redirect til /

async fn index() -> impl IntoResponse {
    render(html! {
        <Page active_page=Page::None>
            <div>
                <form action="/auth/login" method="post">
                    <label for="secret">Secret:</label>
                    <input type="password" id="secret" name="secret" />
                    <button type="submit">Submit</button>
                </form>
            </div>
        </Page>
    })
}

#[derive(Deserialize)]
struct LoginParameters {
    secret: String,
}

async fn login(Form(parameters): Form<LoginParameters>) -> impl IntoResponse {
    let secret = SECRET.get().unwrap();
    println!(
        "login: {}, auth: {}",
        parameters.secret,
        secret == &parameters.secret
    );
}
