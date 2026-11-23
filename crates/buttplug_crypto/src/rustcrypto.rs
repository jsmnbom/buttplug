// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! RustCrypto backend, for every target except ESP-IDF.

use crate::PaddingError;
use aes::Aes128;
use ecb::cipher::block_padding::Pkcs7;
use ecb::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyInit};
use sha2::Digest;

pub fn sha256(data: &[u8]) -> [u8; 32] {
  sha2::Sha256::digest(data).into()
}

pub fn md5(data: &[u8]) -> [u8; 16] {
  md5::Md5::digest(data).into()
}

pub fn aes128_ecb_pkcs7_encrypt(key: &[u8; 16], data: &[u8]) -> Vec<u8> {
  ecb::Encryptor::<Aes128>::new(&(*key).into()).encrypt_padded_vec::<Pkcs7>(data)
}

pub fn aes128_ecb_pkcs7_decrypt(key: &[u8; 16], data: &[u8]) -> Result<Vec<u8>, PaddingError> {
  ecb::Decryptor::<Aes128>::new(&(*key).into())
    .decrypt_padded_vec::<Pkcs7>(data)
    .map_err(|_| PaddingError)
}
