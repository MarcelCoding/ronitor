extern crate core;

use std::{env, thread};
use std::time::Duration;

use rppal::gpio::{Gpio, Mode};
use rppal::hal::Delay;

use dht_embedded::{Dht22, NoopInterruptControl};

use crate::dht_embedded::DhtSensor;

mod dht_embedded;

fn main() -> anyhow::Result<()> {
  let pin_nbr = if let Some(pin_nbr) = env::args().nth(1) {
    pin_nbr.parse()?
  } else {
    panic!("Missing pin number");
  };

  let pin = Gpio::new()?.get(pin_nbr)?.into_io(Mode::Output);

  let mut sensor = Dht22::new(NoopInterruptControl, Delay, pin);

  loop {
    match sensor.read() {
      Ok(reading) => println!("{}°C, {}% RH", reading.temperature(), reading.humidity()),
      Err(e) => eprintln!("Error: {}", e),
    }

    thread::sleep(Duration::from_millis(2100));
  }
}
