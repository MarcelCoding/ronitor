// https://github.com/kelnos/dht-embedded-rs: forked to fixed different dependency versions
// https://cdn-shop.adafruit.com/datasheets/Digital+humidity+and+temperature+sensor+AM2302.pdf

use core::fmt;
use std::time::{Duration, Instant};

use rppal::gpio::{IoPin, Level, Mode};

#[derive(Debug)]
pub(crate) struct Reading {
  pub(crate) humidity: f32,
  pub(crate) temperature: f32,
}

/// A type detailing various errors the DHT sensor can return
#[derive(Debug, Clone)]
pub(crate) enum DhtError {
  /// The DHT sensor was not found on the specified GPIO
  NotPresent,
  /// The checksum provided in the DHT sensor data did not match the checksum of the data itself (expected, calculated)
  ChecksumMismatch(u8, u8),
  /// The seemingly-valid data has impossible values (e.g. a humidity value less than 0 or greater than 100)
  InvalidData,
  /// The read timed out
  TimeoutPost50us(usize),
  TimeoutBit(usize),
}

impl fmt::Display for DhtError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      DhtError::NotPresent => write!(f, "DHT device not found"),
      DhtError::ChecksumMismatch(expected, calculated) => write!(
        f,
        "Data read was corrupt (expected checksum {expected:x}, calculated {calculated:x})",
      ),
      DhtError::InvalidData => f.write_str("Received data is out of range"),
      DhtError::TimeoutPost50us(bit) => write!(f, "Timed out waiting for a read: post 50us, {bit}"),
      DhtError::TimeoutBit(bit) => write!(f, "Timed out waiting for a read: bit, {bit}"),
    }
  }
}

fn sleep(duration: Duration) -> Instant {
  let now = Instant::now();
  while now.elapsed() < duration {}
  now
}

pub(crate) struct Dht22 {
  pin: IoPin,
}

impl Dht22 {
  pub fn new(pin: IoPin) -> Self {
    Self { pin }
  }

  pub(crate) fn read(&mut self) -> Result<Reading, DhtError> {
    // TODO: disable interrupts

    let mut buf: [u8; 5] = [0; 5];

    self.wake_up()?;

    // Now read 40 data bits
    for bit in 0..40 {
      // Wait for high, which takes ~50us
      self.wait_for_level(
        Level::High,
        Duration::from_micros(70),
        DhtError::TimeoutPost50us(bit),
      )?;

      // See how long it takes to go low, with max of 70us (+ ~50us from above)
      let elapsed = self.wait_for_level(
        Level::Low,
        Duration::from_micros(90),
        DhtError::TimeoutBit(bit),
      )?;

      // If it took at least 30us to go low, it's a '1' bit
      if elapsed.as_micros() > 30 {
        let byte = bit / 8;
        let shift = 7 - bit % 8;
        let mask = 1 << shift;
        buf[byte] |= mask;
      }
    }

    let checksum = Self::calc_checksum(&buf);

    if buf[4] != checksum {
      return Err(DhtError::ChecksumMismatch(buf[4], checksum));
    }

    let (humidity, temperature) = Self::parse_data(&buf);

    if !(0.0..=100.0).contains(&humidity) {
      return Err(DhtError::InvalidData);
    }

    Ok(Reading {
      humidity,
      temperature,
    })
  }

  fn wake_up(&mut self) -> Result<(), DhtError> {
    // wake up
    self.pin.set_mode(Mode::Output);
    self.pin.set_high();

    sleep(Duration::from_millis(100));

    // Ask for data
    self.pin.set_low();

    sleep(Duration::from_millis(5));

    self.pin.set_high();

    self.pin.set_mode(Mode::Input);

    // Wait for DHT to signal data is ready (~80us low followed by ~80us high)
    self.wait_for_level(Level::Low, Duration::from_micros(45), DhtError::NotPresent)?;
    self.wait_for_level(Level::High, Duration::from_micros(85), DhtError::NotPresent)?;
    self.wait_for_level(Level::Low, Duration::from_micros(85), DhtError::NotPresent)?;

    Ok(())
  }

  fn wait_for_level(
    &mut self,
    level: Level,
    timeout: Duration,
    on_timeout: DhtError,
  ) -> Result<Duration, DhtError> {
    let start = Instant::now();

    while self.pin.read() != level {
      if start.elapsed() > timeout {
        return Err(on_timeout);
      }
      sleep(Duration::from_micros(1));
    }

    Ok(start.elapsed())
  }

  fn calc_checksum(buf: &[u8; 5]) -> u8 {
    (buf[0..=3]
      .iter()
      .fold(0u16, |accum, next| accum + *next as u16)
      & 0xff) as u8
  }

  fn parse_data(buf: &[u8]) -> (f32, f32) {
    let humidity0 = (buf[0] as u16) << 8;
    let humidity1 = buf[1] as u16;
    let temperature0 = ((buf[2] & 0x7f) as u16) << 8; // 0x7f bit mask: exclude signing bit
    let temperature1 = buf[3] as u16;

    let humidity = (humidity0 | humidity1) as f32 / 10.0;
    let mut temperature = (temperature0 | temperature1) as f32 / 10.0;

    // signing bit
    if buf[2] & 0x80 != 0 {
      temperature = -temperature;
    }

    (humidity, temperature)
  }
}
