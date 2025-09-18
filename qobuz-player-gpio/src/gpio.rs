use qobuz_player_controls::{Result, StatusReceiver, error::Error};
use rppal::gpio::Gpio;

const GPIO: u8 = 23;

pub async fn init(mut status_receiver: StatusReceiver) -> Result<()> {
    let mut pin = Gpio::new()
        .or(Err(Error::GpioUnavailable { pin: GPIO }))?
        .get(GPIO)?
        .into_output();
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
