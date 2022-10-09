use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub(crate) struct Config {
  pub(crate) dht22: Vec<Dht22Config>,
  pub(crate) ds18b22: Vec<Ds18b22Config>,
}

#[derive(Deserialize)]
pub(crate) struct Dht22Config {
  pub(crate) pin: u8,
  pub(crate) temperature_id: Uuid,
  pub(crate) humidity_id: Uuid,
}

#[derive(Deserialize)]
pub(crate) struct Ds18b22Config {
  pub(crate) device_id: String,
  pub(crate) temperature_id: Uuid,
}
