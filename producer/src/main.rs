use std::thread;
use std::time::Duration;

use rppal::gpio::{Gpio,  Mode};
use rppal::hal::Delay;

use dht_embedded::{Dht22, NoopInterruptControl};

use crate::dht_embedded::DhtSensor;

mod dht_embedded;

const GPIO_PIN: u8 = 17;

fn main() -> anyhow::Result<()> {
  let pin = Gpio::new()?.get(GPIO_PIN)?.into_io(Mode::Output);

  let mut sensor = Dht22::new(NoopInterruptControl, Delay, pin);

  loop {
    match sensor.read() {
      Ok(reading) => println!("{}°C, {}% RH", reading.temperature(), reading.humidity()),
      Err(e) => eprintln!("Error: {}", e),
    }

    thread::sleep(Duration::from_millis(2100));
  }
}
