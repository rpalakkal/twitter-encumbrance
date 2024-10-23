use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;
use tokio::sync::{oneshot, Mutex};
use tower_http::cors::CorsLayer;
use twitter::{auth::TwitterTokenPair, builder::TwitterBuilder};

mod event_loop;
mod twitter;

#[derive(Clone)]
pub struct SharedState {
    tee_url: String,
    twitter_builder: TwitterBuilder,
    twitter_token_pair: Arc<Mutex<Option<TwitterTokenPair>>>,
    shutdown_sender: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    oauth_token: String,
    oauth_verifier: String,
}

pub async fn login(State(shared_state): State<SharedState>) -> Redirect {
    let callback_url = format!("{}/callback", shared_state.tee_url.clone(),);

    let oauth_tokens = shared_state
        .twitter_builder
        .request_oauth_token(callback_url)
        .await
        .expect("Failed to request oauth token");

    let mut db = shared_state.twitter_token_pair.lock().await;
    *db = Some(oauth_tokens.clone());

    let url = format!(
        "https://api.twitter.com/oauth/authenticate?oauth_token={}",
        oauth_tokens.token
    );

    Redirect::temporary(&url)
}

pub async fn callback(
    State(shared_state): State<SharedState>,
    Query(query): Query<CallbackQuery>,
) -> String {
    let oauth_token = query.oauth_token;
    let oauth_verifier = query.oauth_verifier;

    let twitter_token_pair = shared_state
        .twitter_token_pair
        .lock()
        .await
        .clone()
        .unwrap();

    assert_eq!(oauth_token, twitter_token_pair.token);

    let token_pair = shared_state
        .twitter_builder
        .authorize_token(
            twitter_token_pair.token.clone(),
            twitter_token_pair.secret.clone(),
            oauth_verifier,
        )
        .await
        .unwrap();

    let mut db = shared_state.twitter_token_pair.lock().await;
    *db = Some(token_pair.clone());

    let twitter_client = shared_state.twitter_builder.with_auth(token_pair);
    let x_info = twitter_client
        .get_user_info()
        .await
        .expect("Failed to get user info");

    if let Some(sender) = shared_state.shutdown_sender.lock().await.take() {
        let _ = sender.send(());
    }

    let msg = format!("Succesfully logged into {}", x_info.name);
    msg
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();

    let tee_url = std::env::var("TEE_URL").expect("TEE_URL not set");
    let consumer_key = std::env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set");
    let consumer_secret =
        std::env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set");

    let twitter_builder = TwitterBuilder::new(consumer_key, consumer_secret);

    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let shared_state = SharedState {
        tee_url,
        twitter_builder: twitter_builder.clone(),
        twitter_token_pair: Arc::new(Mutex::new(None)),
        shutdown_sender: Arc::new(Mutex::new(Some(shutdown_sender))),
    };

    let app = axum::Router::new()
        .route("/login", axum::routing::get(login))
        .route("/callback", axum::routing::get(callback))
        .layer(CorsLayer::permissive())
        .with_state(shared_state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let server = axum::serve(listener, app);
    server
        .with_graceful_shutdown(async {
            shutdown_receiver.await.ok();
        })
        .await
        .ok();
    log::info!("Received credentials. Shutting down server.");

    let tokens = shared_state.twitter_token_pair.lock().await.take().unwrap();
    let twitter_client = shared_state.twitter_builder.with_auth(tokens);
    event_loop::event_loop(twitter_client).await.unwrap();
}
