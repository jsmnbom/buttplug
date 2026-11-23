// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::ServerDeviceMessageAttributesV3;
use super::v2::ServerDeviceMessageAttributesV2;
use buttplug_core::errors::ButtplugError;
use buttplug_server_device_config::ServerDeviceFeature;
use getset::Getters;
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Debug, Getters, Clone)]
pub(crate) struct ServerDeviceAttributes {
  // The v2 and v3 attributes are only built when a message of those spec versions asks for them:
  // a v4 session never pays for them, and without such messages the code is not linked at all.
  attrs_v2: OnceLock<ServerDeviceMessageAttributesV2>,
  attrs_v3: OnceLock<ServerDeviceMessageAttributesV3>,
  #[getset(get = "pub")]
  features: BTreeMap<u32, ServerDeviceFeature>,
}

impl ServerDeviceAttributes {
  pub fn new(features: &BTreeMap<u32, ServerDeviceFeature>) -> Self {
    Self {
      attrs_v2: OnceLock::new(),
      attrs_v3: OnceLock::new(),
      features: features.clone(),
    }
  }

  fn feature_vec(&self) -> Vec<ServerDeviceFeature> {
    self.features.values().cloned().collect()
  }

  pub fn attrs_v2(&self) -> &ServerDeviceMessageAttributesV2 {
    self
      .attrs_v2
      .get_or_init(|| ServerDeviceMessageAttributesV2::from(self.feature_vec()))
  }

  pub fn attrs_v3(&self) -> &ServerDeviceMessageAttributesV3 {
    self
      .attrs_v3
      .get_or_init(|| ServerDeviceMessageAttributesV3::from(self.feature_vec()))
  }
}

pub(crate) trait TryFromClientMessage<T>
where
  Self: Sized,
{
  fn try_from_client_message(
    msg: T,
    features: &BTreeMap<u32, ServerDeviceAttributes>,
  ) -> Result<Self, ButtplugError>;
}
