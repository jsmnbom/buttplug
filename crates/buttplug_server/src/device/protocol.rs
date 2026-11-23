// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Implementations of communication protocols for hardware supported by Buttplug

use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{InputReadingV4, InputType, InputValue, OutputCommand},
};
use buttplug_server_device_config::{
  DeviceConfigurationManager,
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};

use super::hardware::HardwareWriteCmd;
use crate::{
  device::hardware::{Hardware, HardwareCommand, HardwareReadCmd},
  message::{
    ButtplugServerDeviceMessage,
    checked_output_cmd::CheckedOutputCmdV4,
    spec_enums::ButtplugDeviceCommandMessageUnionV4,
  },
};
use async_trait::async_trait;
use futures::{
  StreamExt,
  future::{self, BoxFuture, FutureExt},
};
use std::sync::Arc;
use std::{pin::Pin, time::Duration};
use uuid::Uuid;

/// Strategy for situations where hardware needs to get updates every so often in order to keep
/// things alive. Currently this applies to iOS backgrounding with bluetooth devices, as well as
/// some protocols like Satisfyer and Mysteryvibe that need constant command refreshing, but since
/// we never know which of our hundreds of supported devices someone might connect, we need context
/// as to which keepalive strategy to use.
///
/// When choosing a keepalive strategy for a protocol:
///
/// - If the protocol has a command that essentially does nothing to the actuators, set up
///   RepeatPacketStrategy to use that. This is useful for devices that have info commands (like
///   Lovense), ping commands (like The Handy), sensor commands that aren't yet subscribed to output
///   notifications, etc...
/// - If a protocol needs specific timing or keepalives, regardless of the OS/hardware manager being
///   used, like Satisfyer or Mysteryvibe, use RepeatLastPacketStrategyWithTiming.
/// - For many devices with only scalar actuators, RepeatLastPacketStrategy should work. You just
///   need to make sure the protocol doesn't have a packet counter or something else that will trip
///   if the same packet is replayed multiple times.
#[derive(Debug)]
pub enum ProtocolKeepaliveStrategy {
  /// Repeat a specific packet, such as a ping or a no-op. Only do this when the hardware manager
  /// requires it (currently only iOS bluetooth during backgrounding).
  HardwareRequiredRepeatPacketStrategy(HardwareWriteCmd),
  /// Repeat whatever the last packet sent was, and send Stop commands until first packet sent. Uses
  /// a default timing, suitable for most protocols that don't need constant device updates outside
  /// of OS requirements. Only do this when the hardware manager requires it (currently only iOS
  /// bluetooth during backgrounding).
  HardwareRequiredRepeatLastPacketStrategy,
  /// Repeat whatever the last packet sent was, and send Stop commands until first packet sent. Do
  /// this regardless of whether or not the hardware manager requires it. Useful for hardware that
  /// requires keepalives, like Satisfyer, Mysteryvibe, Leten, etc...
  RepeatLastPacketStrategyWithTiming(Duration),
}

pub enum ProtocolValueCommandPrefilterStrategy {
  /// Drop repeated ValueCmd/ValueWithParameterCmd messages
  DropRepeats,
  /// No filter, send all value messages for processing
  None,
}

fn print_type_of<T>(_: &T) -> &'static str {
  std::any::type_name::<T>()
}

pub struct ProtocolSpecializer {
  specifiers: Vec<ProtocolCommunicationSpecifier>,
  identifier: Box<dyn ProtocolIdentifier>,
}

impl ProtocolSpecializer {
  pub fn new(
    specifiers: Vec<ProtocolCommunicationSpecifier>,
    identifier: Box<dyn ProtocolIdentifier>,
  ) -> Self {
    Self {
      specifiers,
      identifier,
    }
  }

  pub fn specifiers(&self) -> &Vec<ProtocolCommunicationSpecifier> {
    &self.specifiers
  }

  pub fn identify(self) -> Box<dyn ProtocolIdentifier> {
    self.identifier
  }
}

/// Finds the protocol implementation for a protocol name from the device configuration.
///
/// The device manager calls it for every protocol whose communication specifiers match a found
/// device; see `ServerDeviceManagerBuilder::protocol_lookup`. With the `builtin-protocols`
/// feature, [`get_protocol_identifier`](crate::device::protocol_impl::get_protocol_identifier)
/// covers every protocol. A lookup that only names some protocols, each through
/// [`protocol_factory!`](crate::protocol_factory), lets the linker drop all the others.
pub type ProtocolLookup = fn(&str) -> Option<Box<dyn ProtocolIdentifier>>;

#[async_trait]
pub trait ProtocolIdentifier: Sync + Send {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    specifier: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError>;
}

#[async_trait]
pub trait ProtocolInitializer: Sync + Send {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    device_definition: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError>;
}

pub struct GenericProtocolIdentifier<T: ProtocolHandler + Default + 'static> {
  protocol_identifier: &'static str,
  marker: std::marker::PhantomData<T>,
}

impl<T: ProtocolHandler + Default + 'static> GenericProtocolIdentifier<T> {
  pub fn new(protocol_identifier: &'static str) -> Self {
    Self {
      protocol_identifier,
      marker: std::marker::PhantomData,
    }
  }
}

#[async_trait]
impl<T: ProtocolHandler + Default + 'static> ProtocolIdentifier for GenericProtocolIdentifier<T> {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let device_identifier = UserDeviceIdentifier::new(
      hardware.address(),
      self.protocol_identifier,
      &Some(hardware.name().to_owned()),
    );
    Ok((
      device_identifier,
      Box::new(GenericProtocolInitializer::new(Arc::new(T::default()))),
    ))
  }
}

pub struct GenericProtocolInitializer {
  handler: Option<Arc<dyn ProtocolHandler>>,
}

impl GenericProtocolInitializer {
  pub fn new(handler: Arc<dyn ProtocolHandler>) -> Self {
    Self {
      handler: Some(handler),
    }
  }
}

#[async_trait]
impl ProtocolInitializer for GenericProtocolInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(self.handler.take().unwrap())
  }
}

/// Gives the default methods of [ProtocolHandler] the handler as a trait object. Implemented for
/// every handler.
pub trait AsDynProtocolHandler {
  fn as_dyn_protocol_handler(&self) -> &dyn ProtocolHandler;
}

impl<T: ProtocolHandler> AsDynProtocolHandler for T {
  fn as_dyn_protocol_handler(&self) -> &dyn ProtocolHandler {
    self
  }
}

// The default methods of ProtocolHandler are compiled once per handler type, and every one of
// them is kept for every handler through its vtable. Their bodies therefore live in the functions
// below, which exist once, and the defaults only call them.

// Allow here since this changes between debug/release
#[allow(unused_variables)]
#[inline(never)]
fn command_unimplemented(command: &str) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
  #[cfg(debug_assertions)]
  unimplemented!("Command not implemented for this protocol");
  #[cfg(not(debug_assertions))]
  Err(ButtplugDeviceError::UnhandledCommand(format!(
    "Command not implemented for this protocol: {}",
    command
  )))
}

/// The default of the per-actuator output handlers. Takes an index and not the name, and is never
/// inlined: the callers exist once per handler type, so every byte in them counts 150 times.
#[inline(never)]
fn output_unimplemented(actuator: usize) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
  const NAMES: [&str; 9] = [
    "OutputCmd (Vibrate Actuator)",
    "OutputCmd (Rotate Actuator)",
    "OutputCmd (Oscillate Actuator)",
    "OutputCmd (Spray Actuator)",
    "OutputCmd (Constrict Actuator)",
    "OutputCmd (Temperature Actuator)",
    "OutputCmd (Led Actuator)",
    "OutputCmd (Position Actuator)",
    "OutputCmd (Position w/ Duration Actuator)",
  ];
  command_unimplemented(NAMES[actuator])
}

#[inline(never)]
fn unhandled_future<T: Send + 'static>(
  message: &'static str,
) -> BoxFuture<'static, Result<T, ButtplugDeviceError>> {
  future::ready(Err(ButtplugDeviceError::UnhandledCommand(
    message.to_string(),
  )))
  .boxed()
}

/// The default of the input handlers that only refuse. Takes the hardware so that the callers,
/// which exist once per handler type, do not each drop it themselves.
#[inline(never)]
fn input_unimplemented(
  _device: Arc<Hardware>,
  command: usize,
) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
  const MESSAGES: [&str; 3] = [
    "Command not implemented for this protocol: InputCmd (Subscribe)",
    "Command not implemented for this protocol: InputCmd (Unsubscribe)",
    "Command not implemented for this protocol: SensorReadCmd",
  ];
  unhandled_future(MESSAGES[command])
}

fn default_handle_output_cmd(
  handler: &dyn ProtocolHandler,
  cmd: &CheckedOutputCmdV4,
) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
  let output_command = cmd.output_command();
  match output_command {
    OutputCommand::Constrict(x) => handler.handle_output_constrict_cmd(
      cmd.feature_index(),
      cmd.feature_id(),
      x.value()
        .try_into()
        .map_err(|_| ButtplugDeviceError::DeviceCommandSignError)?,
    ),
    OutputCommand::Spray(x) => handler.handle_output_spray_cmd(
      cmd.feature_index(),
      cmd.feature_id(),
      x.value()
        .try_into()
        .map_err(|_| ButtplugDeviceError::DeviceCommandSignError)?,
    ),
    OutputCommand::Oscillate(x) => handler.handle_output_oscillate_cmd(
      cmd.feature_index(),
      cmd.feature_id(),
      x.value()
        .try_into()
        .map_err(|_| ButtplugDeviceError::DeviceCommandSignError)?,
    ),
    OutputCommand::Rotate(x) => {
      handler.handle_output_rotate_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
    }
    OutputCommand::Vibrate(x) => handler.handle_output_vibrate_cmd(
      cmd.feature_index(),
      cmd.feature_id(),
      x.value()
        .try_into()
        .map_err(|_| ButtplugDeviceError::DeviceCommandSignError)?,
    ),
    OutputCommand::Position(x) => handler.handle_output_position_cmd(
      cmd.feature_index(),
      cmd.feature_id(),
      x.value()
        .try_into()
        .map_err(|_| ButtplugDeviceError::DeviceCommandSignError)?,
    ),
    OutputCommand::Temperature(x) => {
      handler.handle_output_temperature_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
    }
    OutputCommand::Led(x) => handler.handle_output_led_cmd(
      cmd.feature_index(),
      cmd.feature_id(),
      x.value()
        .try_into()
        .map_err(|_| ButtplugDeviceError::DeviceCommandSignError)?,
    ),
    OutputCommand::HwPositionWithDuration(x) => handler.handle_hw_position_with_duration_cmd(
      cmd.feature_index(),
      cmd.feature_id(),
      x.value(),
      x.duration(),
    ),
  }
}

fn default_handle_input_read_cmd(
  handler: &dyn ProtocolHandler,
  device_index: u32,
  device: Arc<Hardware>,
  feature_index: u32,
  feature_id: Uuid,
  sensor_type: InputType,
) -> BoxFuture<'_, Result<InputReadingV4, ButtplugDeviceError>> {
  match sensor_type {
    InputType::Battery => {
      handler.handle_battery_level_cmd(device_index, device, feature_index, feature_id)
    }
    _ => unhandled_future("Command not implemented for this protocol: InputCmd (Read)"),
  }
}

fn default_handle_battery_level_cmd(
  device_index: u32,
  device: Arc<Hardware>,
  feature_index: u32,
  feature_id: Uuid,
) -> BoxFuture<'static, Result<InputReadingV4, ButtplugDeviceError>> {
  // If we have a standardized BLE Battery endpoint, handle that above the
  // protocol, as it'll always be the same.
  if device.endpoints().contains(&Endpoint::RxBLEBattery) {
    debug!("Trying to get battery reading.");
    let msg = HardwareReadCmd::new(feature_id, Endpoint::RxBLEBattery, 1, 0);
    let fut = device.read_value(&msg);
    async move {
      let hw_msg = fut.await?;
      let battery_level = hw_msg.data()[0] as i32;
      let battery_reading = InputReadingV4::new(
        device_index,
        feature_index,
        buttplug_core::message::InputTypeReading::Battery(InputValue::new(battery_level as u8)),
      );
      debug!("Got battery reading: {}", battery_level);
      Ok(battery_reading)
    }
    .boxed()
  } else {
    unhandled_future("Command not implemented for this protocol: SensorReadCmd")
  }
}

pub trait ProtocolHandler: AsDynProtocolHandler + Sync + Send {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  // Nothing calls handle_message or command_unimplemented through a trait object: `Self: Sized`
  // keeps them out of the vtable, where each would be kept once per handler type.
  fn handle_message(
    &self,
    message: &ButtplugDeviceCommandMessageUnionV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError>
  where
    Self: Sized,
  {
    command_unimplemented(print_type_of(&message))
  }

  fn command_unimplemented(
    &self,
    command: &str,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError>
  where
    Self: Sized,
  {
    command_unimplemented(command)
  }

  // The default scalar handler assumes that most devices require discrete commands per feature. If
  // a protocol has commands that combine multiple features, either with matched or unmatched
  // actuators, they should just implement their own version of this method.
  fn handle_output_cmd(
    &self,
    cmd: &CheckedOutputCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    default_handle_output_cmd(self.as_dyn_protocol_handler(), cmd)
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    output_unimplemented(0)
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    output_unimplemented(1)
  }

  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    output_unimplemented(2)
  }

  fn handle_output_spray_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    output_unimplemented(3)
  }

  fn handle_output_constrict_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    output_unimplemented(4)
  }

  fn handle_output_temperature_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    output_unimplemented(5)
  }

  fn handle_output_led_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    output_unimplemented(6)
  }

  fn handle_output_position_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _position: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    output_unimplemented(7)
  }

  fn handle_hw_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _position: u32,
    _duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    output_unimplemented(8)
  }

  fn handle_input_subscribe_cmd(
    &self,
    _device_index: u32,
    device: Arc<Hardware>,
    _feature_index: u32,
    _feature_id: Uuid,
    _sensor_type: InputType,
  ) -> BoxFuture<'_, Result<(), ButtplugDeviceError>> {
    input_unimplemented(device, 0)
  }

  fn handle_input_unsubscribe_cmd(
    &self,
    device: Arc<Hardware>,
    _feature_index: u32,
    _feature_id: Uuid,
    _sensor_type: InputType,
  ) -> BoxFuture<'_, Result<(), ButtplugDeviceError>> {
    input_unimplemented(device, 1)
  }

  fn handle_input_read_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> BoxFuture<'_, Result<InputReadingV4, ButtplugDeviceError>> {
    default_handle_input_read_cmd(
      self.as_dyn_protocol_handler(),
      device_index,
      device,
      feature_index,
      feature_id,
      sensor_type,
    )
  }

  // Used for clients that have inputs which can be subscribed. Subscribed inputs should be
  // disconnected between client sessions, but unlike keepalives are not needed to keep the
  // system up.
  fn handle_client_disconnect_cmd(&self) -> BoxFuture<'_, Result<(), ButtplugDeviceError>> {
    future::ready(Ok(())).boxed()
  }

  // Handle Battery Level returns a SensorReading, as we'll always need to do a sensor index
  // conversion on it.
  fn handle_battery_level_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<'_, Result<InputReadingV4, ButtplugDeviceError>> {
    default_handle_battery_level_cmd(device_index, device, feature_index, feature_id)
  }

  fn handle_rssi_level_cmd(
    &self,
    device: Arc<Hardware>,
    _feature_index: u32,
    _feature_id: Uuid,
  ) -> BoxFuture<'_, Result<(), ButtplugDeviceError>> {
    input_unimplemented(device, 2)
  }

  fn event_stream(
    &self,
  ) -> Pin<Box<dyn tokio_stream::Stream<Item = ButtplugServerDeviceMessage> + Send>> {
    tokio_stream::empty().boxed()
  }
}

#[macro_export]
macro_rules! generic_protocol_setup {
  ( $protocol_name:ident, $protocol_identifier:tt) => {
    pub mod setup {
      use $crate::device::protocol::{GenericProtocolIdentifier, ProtocolIdentifier};

      pub const IDENTIFIER: &str = $protocol_identifier;

      pub fn create_identifier() -> Box<dyn ProtocolIdentifier> {
        Box::new(GenericProtocolIdentifier::<super::$protocol_name>::new(
          IDENTIFIER,
        ))
      }
    }
  };
}

#[macro_export]
macro_rules! generic_protocol_initializer_setup {
  ( $protocol_name:ident, $protocol_identifier:tt) => {
    paste::paste! {
      // This macro used to generate an identifier type per protocol, and the protocol files
      // import what that needed. The shared GenericProtocolIdentifier does not need it.
      #[allow(unused_imports)]
      use self::{
        ProtocolCommunicationSpecifier as _,
        ProtocolIdentifier as _,
        UserDeviceIdentifier as _,
      };

      pub mod setup {
        use $crate::device::protocol::ProtocolIdentifier;

        pub const IDENTIFIER: &str = $protocol_identifier;

        pub fn create_identifier() -> Box<dyn ProtocolIdentifier> {
          Box::new(super::[< $protocol_name Identifier >]::default())
        }
      }

      #[derive(Default)]
      pub struct [< $protocol_name Identifier >] {}

      #[async_trait]
      impl ProtocolIdentifier for [< $protocol_name Identifier >] {
        async fn identify(
          &mut self,
          hardware: Arc<Hardware>,
          _: ProtocolCommunicationSpecifier,
        ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
          Ok((UserDeviceIdentifier::new(hardware.address(), $protocol_identifier, &Some(hardware.name().to_owned())), Box::new([< $protocol_name Initializer >]::default())))
        }
      }
    }
  };
}

pub use generic_protocol_initializer_setup;
pub use generic_protocol_setup;

pub fn get_protocol_specializers(
  specifier: &ProtocolCommunicationSpecifier,
  device_config_manager: &DeviceConfigurationManager,
  protocol_lookup: ProtocolLookup,
) -> Vec<ProtocolSpecializer> {
  debug!(
    "Looking for protocol that matches specifier: {:?}",
    specifier
  );
  let mut specializers = vec![];
  let mut add_specializer = |name: &str, specifiers: Vec<ProtocolCommunicationSpecifier>| {
    info!(
      "Found protocol {:?} for user specifier {:?}.",
      name, specifier
    );
    if let Some(identifier) = protocol_lookup(name) {
      specializers.push(ProtocolSpecializer::new(specifiers, identifier));
    } else {
      warn!(
        "No protocol implementation for {:?} found for specifier {:?}.",
        name, specifier
      );
    }
  };
  for spec in device_config_manager.user_communication_specifiers().iter() {
    if spec.value().contains(specifier) {
      add_specializer(spec.key(), spec.value().clone());
    }
  }
  for (name, specifiers) in device_config_manager.matching_base_communication_specifiers(specifier)
  {
    add_specializer(&name, specifiers);
  }
  specializers
}

/*
#[cfg(test)]
mod test {
  use super::*;
  use crate::{
    core::message::{OutputType, FeatureType},
    server::message::server_device_feature::{ServerDeviceFeature, ServerDeviceFeatureOutput},
  };
  use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
  };

  fn create_unit_test_dcm() -> DeviceConfigurationManager {
    let mut builder = DeviceConfigurationManagerBuilder::default();
    let specifiers = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new(
      HashSet::from(["LVS-*".to_owned(), "LovenseDummyTestName".to_owned()]),
      vec![],
      HashSet::new(),
      HashMap::new(),
    ));
    let mut feature_actuator = HashMap::new();
    feature_actuator.insert(
      OutputType::Vibrate,
      ServerDeviceFeatureOutput::new(&RangeInclusive::new(0, 20), &RangeInclusive::new(0, 20)),
    );
    builder
      .communication_specifier("lovense", &[specifiers])
      .protocol_features(
        &BaseDeviceIdentifier::new("lovense", &Some("P".to_owned())),
        &BaseDeviceDefinition::new(
          "Lovense Edge",
          &uuid::Uuid::new_v4(),
          &None,
          &vec![
            ServerDeviceFeature::new(
              "Edge Vibration 1",
              &uuid::Uuid::new_v4(),
              &None,
              FeatureType::Vibrate,
              &Some(feature_actuator.clone()),
              &None,
            ),
            ServerDeviceFeature::new(
              "Edge Vibration 2",
              &uuid::Uuid::new_v4(),
              &None,
              FeatureType::Vibrate,
              &Some(feature_actuator.clone()),
              &None,
            ),
          ],
          &None
        ),
      )
      .finish()
      .unwrap()
  }

  #[test]
  fn test_config_equals() {
    let config = create_unit_test_dcm();
    let spec = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Something",
      &HashMap::new(),
      &[],
    ));
    assert!(!config.protocol_specializers(&spec).is_empty());
  }

  #[test]
  fn test_config_wildcard_equals() {
    let config = create_unit_test_dcm();
    let spec = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &HashMap::new(),
      &[],
    ));
    assert!(!config.protocol_specializers(&spec).is_empty());
  }
  /*
  #[test]
  fn test_specific_device_config_creation() {
    let dcm = create_unit_test_dcm(false);
    let spec = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &HashMap::new(),
      &[],
    ));
    assert!(!dcm.protocol_specializers(&spec).is_empty());
    let config: ProtocolDeviceAttributes = dcm
      .device_definition(
        &UserDeviceIdentifier::new("Whatever", "lovense", &Some("P".to_owned())),
        &[],
      )
      .expect("Should be found")
      .into();
    // Make sure we got the right name
    assert_eq!(config.name(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert_eq!(
      config
        .message_attributes()
        .scalar_cmd()
        .as_ref()
        .expect("Test, assuming infallible")
        .get(0)
        .expect("Test, assuming infallible")
        .step_count(),
      20
    );
  }
  */
}
*/
