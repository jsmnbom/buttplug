// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! The few crypto primitives Buttplug device protocols use, as plain functions.
//!
//! Two backends with the same behaviour:
//!
//! - RustCrypto (`sha2`, `md-5`, `aes` + `ecb`), on every target except ESP-IDF.
//! - ESP-IDF, under `cfg(target_os = "espidf")`, so a firmware does not carry a second copy of
//!   primitives the chip and ESP-IDF already have: mbedTLS SHA-256 (hardware SHA), the hardware
//!   AES driver, and the MD5 in the chip's ROM. Only those functions get linked.
//!
//! The ESP-IDF calls can only fail on a driver failure (no DMA memory, for instance), which
//! the RustCrypto backend has no equivalent of; that panics rather than adding an error the
//! protocols would have to handle only on one target.

#[cfg(target_os = "espidf")]
mod espidf;
#[cfg(target_os = "espidf")]
use espidf as backend;
#[cfg(any(target_os = "espidf", test))]
mod pkcs7;
#[cfg(not(target_os = "espidf"))]
mod rustcrypto;
#[cfg(not(target_os = "espidf"))]
use rustcrypto as backend;

/// A ciphertext did not decrypt to correctly PKCS#7 padded data: it is empty, its length is
/// not a multiple of the block size, or the padding bytes are wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaddingError;

impl std::fmt::Display for PaddingError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("Padding error")
  }
}

impl std::error::Error for PaddingError {
}

/// SHA-256 of `data`.
pub fn sha256(data: &[u8]) -> [u8; 32] {
  backend::sha256(data)
}

/// MD5 of `data`.
pub fn md5(data: &[u8]) -> [u8; 16] {
  backend::md5(data)
}

/// AES-128 in ECB mode with PKCS#7 padding. The result is always 1 to 16 bytes longer than
/// `data`, a whole number of 16-byte blocks.
pub fn aes128_ecb_pkcs7_encrypt(key: &[u8; 16], data: &[u8]) -> Vec<u8> {
  backend::aes128_ecb_pkcs7_encrypt(key, data)
}

/// Reverses [`aes128_ecb_pkcs7_encrypt`]. Padding is checked strictly (every padding byte),
/// as RustCrypto's `Pkcs7` does.
pub fn aes128_ecb_pkcs7_decrypt(key: &[u8; 16], data: &[u8]) -> Result<Vec<u8>, PaddingError> {
  backend::aes128_ecb_pkcs7_decrypt(key, data)
}
