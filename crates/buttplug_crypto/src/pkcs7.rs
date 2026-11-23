// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! ECB mode with PKCS#7 padding over a single-block cipher, for backends that only offer the
//! block operation. Behaves like RustCrypto's `encrypt_padded_vec::<Pkcs7>` /
//! `decrypt_padded_vec::<Pkcs7>`; the tests below check that against RustCrypto.

use crate::PaddingError;

const BLOCK: usize = 16;

pub fn encrypt(data: &[u8], mut encrypt_block: impl FnMut(&mut [u8; BLOCK])) -> Vec<u8> {
  let pad = BLOCK - data.len() % BLOCK;
  let mut out = Vec::with_capacity(data.len() + pad);
  out.extend_from_slice(data);
  out.resize(data.len() + pad, pad as u8);
  for block in out.chunks_exact_mut(BLOCK) {
    encrypt_block(block.try_into().expect("exact chunk"));
  }
  out
}

pub fn decrypt(
  data: &[u8],
  mut decrypt_block: impl FnMut(&mut [u8; BLOCK]),
) -> Result<Vec<u8>, PaddingError> {
  if data.is_empty() || data.len() % BLOCK != 0 {
    return Err(PaddingError);
  }
  let mut out = data.to_vec();
  for block in out.chunks_exact_mut(BLOCK) {
    decrypt_block(block.try_into().expect("exact chunk"));
  }
  let pad = usize::from(out[out.len() - 1]);
  if pad == 0
    || pad > BLOCK
    || out[out.len() - pad..]
      .iter()
      .any(|&b| usize::from(b) != pad)
  {
    return Err(PaddingError);
  }
  out.truncate(out.len() - pad);
  Ok(out)
}

#[cfg(all(test, not(target_os = "espidf")))]
mod tests {
  use aes::Aes128;
  use aes::cipher::{BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};

  const KEY: [u8; 16] = *b"jdk#Cra%f5Vib28r";

  fn encrypt(data: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new(&KEY.into());
    super::encrypt(data, |block| {
      let mut b = aes::Block::from(*block);
      cipher.encrypt_block(&mut b);
      block.copy_from_slice(&b);
    })
  }

  fn decrypt(data: &[u8]) -> Result<Vec<u8>, crate::PaddingError> {
    let cipher = Aes128::new(&KEY.into());
    super::decrypt(data, |block| {
      let mut b = aes::Block::from(*block);
      cipher.decrypt_block(&mut b);
      block.copy_from_slice(&b);
    })
  }

  /// The hand-written padding the ESP-IDF backend uses agrees with RustCrypto's, for every
  /// length across three blocks and for every possible last block.
  #[test]
  fn matches_rustcrypto() {
    for len in 0..=48 {
      let data = (0..len as u8)
        .map(|i| i.wrapping_mul(37))
        .collect::<Vec<_>>();
      let ours = encrypt(&data);
      assert_eq!(
        ours,
        crate::rustcrypto::aes128_ecb_pkcs7_encrypt(&KEY, &data),
        "len {len}"
      );
      assert_eq!(decrypt(&ours), Ok(data), "len {len}");
    }
    // Ciphertexts whose last plaintext block ends in every byte value, with the bytes before
    // it set to that value or not: covers valid, short, zero, oversized and mixed padding.
    let cipher = Aes128::new(&KEY.into());
    for last in 0..=255u8 {
      for fill in [last, last ^ 1] {
        for blocks in 1..=2 {
          let mut plain = vec![fill; 16 * blocks];
          *plain.last_mut().unwrap() = last;
          let mut ciphertext = plain.clone();
          for block in ciphertext.chunks_exact_mut(16) {
            let mut b = aes::Block::try_from(&*block).unwrap();
            cipher.encrypt_block(&mut b);
            block.copy_from_slice(&b);
          }
          assert_eq!(
            decrypt(&ciphertext),
            crate::rustcrypto::aes128_ecb_pkcs7_decrypt(&KEY, &ciphertext),
            "last {last:#x} fill {fill:#x} blocks {blocks}"
          );
        }
      }
    }
    for len in [1, 15, 17, 31, 33] {
      let data = vec![0u8; len];
      assert_eq!(decrypt(&data), Err(crate::PaddingError), "len {len}");
      assert!(crate::rustcrypto::aes128_ecb_pkcs7_decrypt(&KEY, &data).is_err());
    }
  }
}
