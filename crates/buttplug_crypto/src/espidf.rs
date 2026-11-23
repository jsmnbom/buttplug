// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! ESP-IDF backend.
//!
//! - SHA-256: `mbedtls_sha256`, the one-shot function; with `CONFIG_MBEDTLS_HARDWARE_SHA` it
//!   runs on the SHA peripheral. One-shot, so no context struct crosses the FFI boundary.
//! - MD5: `esp_rom_md5_*`, the implementation in the chip's ROM. It costs no flash and does not
//!   depend on mbedTLS being configured with MD5.
//! - AES-128: `esp_aes_*`, ESP-IDF's hardware AES driver (what `mbedtls_aes_*` maps to with
//!   `CONFIG_MBEDTLS_HARDWARE_AES`). It takes the hardware lock itself, so it is safe next to
//!   other users of the peripheral (NimBLE, when it uses mbedTLS). ECB and PKCS#7 padding are
//!   done here, one block per call.
//!
//! Each of these pulls in only its own objects, so with nothing else in the firmware using
//! mbedTLS (for instance `CONFIG_BT_NIMBLE_CRYPTO_STACK_MBEDTLS=n`), only they get linked.

use crate::PaddingError;
use core::ffi::c_int;
use esp_idf_sys as sys;

pub fn sha256(data: &[u8]) -> [u8; 32] {
  let mut out = [0; 32];
  // SAFETY: `data` is readable for its length, `out` is the 32 bytes SHA-256 writes.
  let ret = unsafe { sys::mbedtls_sha256(data.as_ptr(), data.len(), out.as_mut_ptr(), 0) };
  assert!(ret == 0, "mbedtls_sha256 failed: {ret}");
  out
}

pub fn md5(data: &[u8]) -> [u8; 16] {
  let mut context = sys::md5_context_t::default();
  let mut out = [0; 16];
  // SAFETY: the context is the ROM's own layout (from the ESP-IDF headers) and is initialized
  // before use; `out` is the 16 bytes MD5 writes.
  unsafe {
    sys::esp_rom_md5_init(&mut context);
    for chunk in data.chunks(u32::MAX as usize) {
      sys::esp_rom_md5_update(&mut context, chunk.as_ptr().cast(), chunk.len() as u32);
    }
    sys::esp_rom_md5_final(out.as_mut_ptr(), &mut context);
  }
  out
}

/// An `esp_aes_context` with a 128-bit key set.
struct Aes128(sys::esp_aes_context);

impl Aes128 {
  fn new(key: &[u8; 16]) -> Self {
    let mut aes = Self(sys::esp_aes_context::default());
    // SAFETY: the context is plain data (key and its length); `key` is 16 bytes.
    let ret = unsafe {
      sys::esp_aes_init(&mut aes.0);
      sys::esp_aes_setkey(&mut aes.0, key.as_ptr(), 128)
    };
    assert!(ret == 0, "esp_aes_setkey failed: {ret}");
    aes
  }

  /// Encrypts or decrypts one block in place. The driver gets copies on this task's stack, so
  /// it never has to bounce a buffer the DMA cannot reach (the Rust heap may be in PSRAM).
  fn block(&mut self, mode: u32, block: &mut [u8; 16]) {
    let input = *block;
    let mut output = [0; 16];
    // SAFETY: `input` and `output` are 16-byte blocks; the context has a key set.
    let ret = unsafe {
      sys::esp_aes_crypt_ecb(
        &mut self.0,
        mode as c_int,
        input.as_ptr(),
        output.as_mut_ptr(),
      )
    };
    assert!(ret == 0, "esp_aes_crypt_ecb failed: {ret}");
    *block = output;
  }
}

impl Drop for Aes128 {
  fn drop(&mut self) {
    // SAFETY: the context was initialized by `esp_aes_init`.
    unsafe { sys::esp_aes_free(&mut self.0) };
  }
}

pub fn aes128_ecb_pkcs7_encrypt(key: &[u8; 16], data: &[u8]) -> Vec<u8> {
  let mut aes = Aes128::new(key);
  crate::pkcs7::encrypt(data, |block| aes.block(sys::ESP_AES_ENCRYPT, block))
}

pub fn aes128_ecb_pkcs7_decrypt(key: &[u8; 16], data: &[u8]) -> Result<Vec<u8>, PaddingError> {
  let mut aes = Aes128::new(key);
  crate::pkcs7::decrypt(data, |block| aes.block(sys::ESP_AES_DECRYPT, block))
}
