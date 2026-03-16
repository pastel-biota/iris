use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, de::DeserializeOwned};
use toml::map::Map;

use crate::ingest::model::Properties;

#[derive(Debug, Deserialize)]
pub struct ProcessorConfig(HashMap<String, toml::Value>);

pub struct ProcessorContext {
    hide_locations: HideLocation,
    hide_params: HideParameter,
}

pub fn create_property_processor_context(
    config: &ProcessorConfig,
) -> Result<ProcessorContext, anyhow::Error> {
    Ok(ProcessorContext {
        hide_locations: parse_config::<HideLocation>(config)?,
        hide_params: parse_config::<HideParameter>(config)?,
    })
}

fn parse_config<T: PropertyProcessor>(config: &ProcessorConfig) -> Result<T, anyhow::Error> {
    let config_values = config
        .0
        .get(T::CONFIG_KEY)
        .cloned()
        .unwrap_or_else(|| toml::Value::Table(Map::new()));

    let config = T::Config::deserialize(config_values)?;

    T::with_config(config)
}

pub fn process_properties(
    context: &ProcessorContext,
    props: Properties,
) -> Result<Properties, anyhow::Error> {
    let props = context.hide_locations.process(props)?;
    let props = context.hide_params.process(props)?;

    Ok(props)
}

pub trait PropertyProcessor: Sized {
    const CONFIG_KEY: &'static str;
    type Config: DeserializeOwned;

    fn with_config(config: Self::Config) -> Result<Self, anyhow::Error>;
    fn process(&self, properties: Properties) -> Result<Properties, anyhow::Error>;
}

pub struct HideLocation(HideLocationConfig);

#[derive(Debug, serde::Deserialize)]
pub struct HideLocationConfig {
    latlngs: Vec<(f64, f64)>,
}

impl PropertyProcessor for HideLocation {
    const CONFIG_KEY: &'static str = "hide_locations";
    type Config = HideLocationConfig;

    fn process(&self, mut properties: Properties) -> Result<Properties, anyhow::Error> {
        let Some(lat_lng) = properties.gps_lat_lng else {
            tracing::debug!("No location was registered to this photo");
            return Ok(properties);
        };

        if self.0.latlngs.iter().any(|masked_location| {
            let dist = ((masked_location.0 - lat_lng.0).powf(2.0)
                + (masked_location.1 - lat_lng.1).powf(2.0))
            .sqrt();
            tracing::trace!("{:?} v. {:?} = {}", masked_location, lat_lng, dist);
            dist <= 0.03
        }) {
            tracing::info!(
                "This location was hidden when registeration: {:?}",
                properties.gps_lat_lng
            );
            properties.gps_lat_lng = None;
        } else {
            tracing::debug!(
                "This location is not hidden when registeration: {:?}",
                properties.gps_lat_lng
            );
        }

        Ok(properties)
    }

    fn with_config(config: Self::Config) -> Result<Self, anyhow::Error> {
        tracing::info!("These location will be hidden if it's nearby (~ 0.03 degs)");
        config.latlngs.iter().for_each(|(lat, lng)| {
            tracing::info!("  - {lat}, {lng}");
        });
        tracing::warn!("Loction is hidden ONLY AT REGISTERATION - reimport when this changed");

        tracing::debug!("Detailed message will be reported");

        Ok(Self(config))
    }
}

pub struct HideParameter(HideParameterConfig);

#[derive(Debug, serde::Deserialize)]
pub struct HideParameterConfig {
    hiding_machine: Vec<String>,
}

impl PropertyProcessor for HideParameter {
    const CONFIG_KEY: &'static str = "hide_params";
    type Config = HideParameterConfig;

    fn process(&self, mut properties: Properties) -> Result<Properties, anyhow::Error> {
        if self
            .0
            .hiding_machine
            .iter()
            .any(|hiding_machine| properties.machine.contains(hiding_machine))
        {
            properties.gps_lat_lng = None;
            properties.lens = None;
            properties.f_number = None;
            properties.shutter_speed = None;
            properties.shutter_speed_controlled = None;
            properties.iso = None;
            properties.focal = None;
        }

        Ok(properties)
    }

    fn with_config(config: Self::Config) -> Result<Self, anyhow::Error> {
        Ok(Self(config))
    }
}
