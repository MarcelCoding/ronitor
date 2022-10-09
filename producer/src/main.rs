extern crate core;

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::io::{BufRead, BufReader};

use anyhow::anyhow;
use reqwest::Client;
use rppal::gpio::{Gpio, Mode};
use serde::Serialize;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;
use uuid::Uuid;

use dht22::Dht22;

use crate::config::Config;
use crate::ds18b20::Ds18b20;

mod config;
mod dht22;
mod ds18b20;

#[derive(Serialize, Debug)]
struct SensorData {
  #[serde(with = "time::serde::iso8601")]
  timestamp: OffsetDateTime,
  value: f32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let args = std::env::args().collect::<Vec<String>>();

  if args.len() != 3 {
    return Err(anyhow!("Please provide a config and cache path."));
  }

  let config: Config = {
    let file = File::open(&args[1])?;
    serde_yaml::from_reader(file)?
  };

  let mut data = Vec::new();
  let now = OffsetDateTime::now_utc();

  for sensor in &config.dht22 {
    let pin = Gpio::new()?.get(sensor.pin)?.into_io(Mode::Input);
    let mut dht22 = Dht22::new(pin);

    let mut errors = 0;

    while errors < 3 {
      match dht22.read() {
        Ok(reading) => {
          data.push((
            sensor.humidity_id,
            SensorData {
              timestamp: now,
              value: reading.humidity,
            },
          ));
          data.push((
            sensor.temperature_id,
            SensorData {
              timestamp: now,
              value: reading.temperature,
            },
          ));
          errors = 10;
        }
        Err(e) => {
          eprintln!(
            "{}/{}: Error: {}",
            sensor.humidity_id, sensor.temperature_id, e
          );
          errors += 1;
        }
      }
    }
  }

  for sensor in &config.ds18b22 {
    let ds18b220 = Ds18b20::new(&sensor.device_id);

    let mut errors = 0;

    while errors < 3 {
      match ds18b220.read() {
        Ok(temperature) => {
          data.push((
            sensor.temperature_id,
            SensorData {
              timestamp: now,
              value: temperature,
            },
          ));
          errors = 10;
        }
        Err(e) => {
          eprintln!("{}: Error: {}", sensor.temperature_id, e);
          errors += 1;
        }
      }
    }
  }

  let client = Client::new();

  let mut errors = Vec::new();

  for (id, data) in data {
    if !push_data(&client, &id, &data).await {
      errors.push((id, data));
    }
  }

  if errors.is_empty() {
    data = Vec::new();

    {
      let mut file = BufReader::new(File::open(&args[2])?);

      let mut line = String::new();
      while file.read_line(&mut line)? != 0 {
        let columns = line.trim().split(';').collect::<Vec<&str>>();
        if columns.len() != 3 {
          continue;
        }

        data.push((
          Uuid::parse_str(columns[0])?,
          SensorData {
            timestamp: OffsetDateTime::parse(columns[1], &Iso8601::DEFAULT)?,
            value: columns[2].parse()?,
          },
        ))
      }
    }

    for (id, data) in data {
      if !push_data(&client, &id, &data).await {
        errors.push((id, data));
      }
    }

    write_to_cache(true, &args[2], errors)?;
  } else {
    write_to_cache(false, &args[2], errors)?;
  }
  Ok(())
}

fn write_to_cache(
  overwrite: bool,
  path: &str,
  entries: Vec<(Uuid, SensorData)>,
) -> anyhow::Result<()> {
  let mut file = OpenOptions::new().write(true).append(true).open(path)?;

  if overwrite {
    file.set_len(0)?;
  }

  for (id, SensorData { timestamp, value }) in entries {
    writeln!(
      file,
      "{};{};{}",
      id,
      timestamp.format(&Iso8601::DEFAULT)?,
      value
    )?;
  }

  Ok(())
}

async fn push_data(client: &Client, id: &Uuid, data: &SensorData) -> bool {
  let url = format!("https://luna.m4rc3l.de/api/weather/sensor/{}/data", &id);

  if let Err(err) = client
    .post(url)
    .json(&data)
    .send()
    .await
    .and_then(|resp| resp.error_for_status())
  {
    eprintln!("Error while publishing data: {err}");
    return false;
  }

  true
}
