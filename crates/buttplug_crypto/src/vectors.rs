// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Known-answer vectors, checked against whichever backend is active.
//!
//! Standard vectors where there are some (FIPS 180-2 for SHA-256, RFC 1321 for MD5, FIPS-197
//! appendix C.1 for AES-128); the rest were computed with Python's `hashlib` and the OpenSSL
//! command line (`openssl enc -aes-128-ecb`, which pads with PKCS#7), not with this crate.

use crate::{PaddingError, aes128_ecb_pkcs7_decrypt, aes128_ecb_pkcs7_encrypt, md5, sha256};

/// 200 bytes, `(i * 7 + 3) & 0xff`: longer than one hash block.
fn long_input() -> Vec<u8> {
  (0..200u32).map(|i| (i * 7 + 3) as u8).collect()
}

const FIPS_KEY: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
const VIBCRAFTER_KEY: [u8; 16] = *b"jdk#Cra%f5Vib28r";
const FLUFFER_KEY: [u8; 16] = *b"jdk#Flu%y6fer32f";

fn hex(bytes: &[u8]) -> String {
  bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(hex: &str) -> Vec<u8> {
  (0..hex.len())
    .step_by(2)
    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("valid hex"))
    .collect()
}

pub fn run() -> Result<usize, String> {
  let mut checks = 0;
  let mut check = |what: &str, ok: bool, detail: String| {
    checks += 1;
    if ok {
      Ok(())
    } else {
      Err(format!("{what}: {detail}"))
    }
  };

  let long = long_input();
  let hashes: [(&str, &[u8], &str); 4] = [
    (
      "empty",
      b"",
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
      "abc",
      b"abc",
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    ),
    (
      "448 bits",
      b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
      "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
    ),
    (
      "200 bytes",
      &long,
      "2c7e18c942ef065b526a2d4e5546283749cd3ddfb51d8fc71f42717363685f46",
    ),
  ];
  for (name, input, expected) in hashes {
    let got = hex(&sha256(input));
    check(&format!("sha256 {name}"), got == expected, got)?;
  }

  let digests: [(&str, &[u8], &str); 5] = [
    ("empty", b"", "d41d8cd98f00b204e9800998ecf8427e"),
    ("abc", b"abc", "900150983cd24fb0d6963f7d28e17f72"),
    (
      "message digest",
      b"message digest",
      "f96b697d7cb7938d525a2f31aaf161d0",
    ),
    (
      "80 digits",
      b"12345678901234567890123456789012345678901234567890123456789012345678901234567890",
      "57edf4a22be3c955ac49da2e2107b67a",
    ),
    ("200 bytes", &long, "4c79b81ac94bad7a875519ce6b964c66"),
  ];
  for (name, input, expected) in digests {
    let got = hex(&md5(input));
    check(&format!("md5 {name}"), got == expected, got)?;
  }

  let fips_plain = unhex("00112233445566778899aabbccddeeff");
  let fluffer_plain = [0xa5, 0x01, 0x08, 1, 2, 3, 4, 5, 6, 7, 8];
  let ciphers: [(&str, &[u8; 16], &[u8], &str); 5] = [
    (
      "FIPS-197 block",
      &FIPS_KEY,
      &fips_plain,
      "69c4e0d86a7b0430d8cdb78070b4c55a954f64f2e4e86e9eee82d20216684899",
    ),
    ("empty", &FIPS_KEY, b"", "954f64f2e4e86e9eee82d20216684899"),
    (
      "vibcrafter 11 bytes",
      &VIBCRAFTER_KEY,
      b"MtInt:0102;",
      "17e3beb301f98e77fd5802f22d1c34d9",
    ),
    (
      "vibcrafter 18 bytes",
      &VIBCRAFTER_KEY,
      b"Auth:0123456789ab;",
      "f47552691b066388f093695d7f099993b919f3a154a72ca206a1033cab949627",
    ),
    (
      "fluffer 11 bytes",
      &FLUFFER_KEY,
      &fluffer_plain,
      "ce788e9077a2a1c2fa651361fdfcf6e8",
    ),
  ];
  for (name, key, plain, expected) in ciphers {
    let got = aes128_ecb_pkcs7_encrypt(key, plain);
    check(
      &format!("aes encrypt {name}"),
      hex(&got) == expected,
      hex(&got),
    )?;
    let back = aes128_ecb_pkcs7_decrypt(key, &got);
    check(
      &format!("aes decrypt {name}"),
      back.as_deref() == Ok(plain),
      format!("{back:?}"),
    )?;
  }

  // Decryption must fail exactly where RustCrypto's does. The bad-padding ciphertexts are
  // single blocks encrypted without padding (`-nopad`) under `FIPS_KEY`.
  let rejected: [(&str, Vec<u8>); 6] = [
    ("empty", Vec::new()),
    ("15 bytes", vec![0; 15]),
    ("17 bytes", vec![0; 17]),
    // Plaintext: 16 zero bytes, so a padding byte of 0.
    ("padding 0", unhex("c6a13b37878f5b826f4f8162a1c8d879")),
    // Plaintext: 15 zero bytes then 0x11, a padding longer than the block.
    ("padding 17", unhex("4493ada3306ce110f48157d8668959d7")),
    // Plaintext: 13 times 'A' then 02 03 03: claims 3 bytes of padding, one is wrong.
    ("mixed padding", unhex("9e6e070fe7acf025e8235e98a20d7f65")),
  ];
  for (name, ciphertext) in rejected {
    let got = aes128_ecb_pkcs7_decrypt(&FIPS_KEY, &ciphertext);
    check(
      &format!("aes rejects {name}"),
      got == Err(PaddingError),
      format!("{got:?}"),
    )?;
  }

  Ok(checks)
}
