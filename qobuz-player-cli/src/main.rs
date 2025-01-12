use std::process;

#[tokio::main]
async fn main() {
    match qobuz_player::cli::run().await {
        Ok(()) => {}
        Err(err) => {
            println!("{err}");
            process::exit(1);
        }
    }
}
