use std::fs::File;
use std::io::Read;

use anyhow::anyhow;

pub(crate) struct Ds18b20<'a> {
  id: &'a str,
}

impl<'a> Ds18b20<'a> {
  pub fn new(id: &'a str) -> Self {
    Self { id }
  }

  pub(crate) fn read(&self) -> anyhow::Result<f32> {
    let path = format!("/sys/devices/w1_bus_master1/{}/w1_slave", self.id);
    let mut file = File::open(path)?;

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    let temperature = Self::parse(&buf)?;
    Ok(temperature)
  }

  fn parse(data: &str) -> anyhow::Result<f32> {
    // 93 01 4b 46 7f ff 0c 10 f6 : crc=f6 YES
    // 93 01 4b 46 7f ff 0c 10 f6 t=25187

    if let Some(line) = data.lines().nth(1) {
      if let Some((_, value_string)) = line.split_once('=') {
        let raw = value_string.parse::<u32>()?;
        let temperature = raw as f32 / 1000_f32;
        return Ok(temperature);
      }
    }

    // TODO: escape data
    Err(anyhow!("Unable to extract temperature from data {}", data))
  }
}
