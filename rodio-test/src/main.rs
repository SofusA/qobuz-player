use std::{env, io::Cursor};

use rodio::Source;

#[tokio::main]
async fn main() {
    let username = env::var("username").unwrap();
    let password = env::var("password").unwrap();

    let client = qobuz_player_client::client::new(&username, &password)
        .await
        .unwrap();

    let track = client.track_url(64868955, None).await.unwrap();

    let resp = reqwest::blocking::get(track.url).unwrap();
    let cursor = Cursor::new(resp.bytes().unwrap());
    // let device = rodio::default_output_device().unwrap();
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let source = rodio::Decoder::new(cursor).unwrap();

    stream_handle.play_raw(source.convert_samples()).unwrap();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}
