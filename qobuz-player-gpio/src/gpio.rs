use qobuz_player_controls::StatusReceiver;
use rppal::gpio::Gpio;

const GPIO: u8 = 23;

pub async fn init(mut status_receiver: StatusReceiver) {
    let mut pin = Gpio::new().unwrap().get(GPIO).unwrap().into_output();
    tracing::info!("Pin claimed");

    loop {
        if status_receiver.changed().await.is_ok() {
            let status = status_receiver.borrow_and_update();
            match *status {
                qobuz_player_controls::Status::Paused => {
                    pin.set_low();
                    tracing::info!("Gpio low");
                }
                qobuz_player_controls::Status::Playing
                | qobuz_player_controls::Status::Buffering => {
                    pin.set_high();
                    tracing::info!("Gpio high");
                }
            }
        }
    }
}
