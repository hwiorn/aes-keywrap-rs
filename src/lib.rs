#![feature(test)]
extern crate test;

use crypto2::blockmode::{Aes128Ecb, Aes192Ecb, Aes256Ecb};
use std::io::Write;

// constants for initial value in primary (RFC3394) and extended (RFC5649) definition
pub const IV_3394: [u8; 8] = [0xa6, 0xa6, 0xa6, 0xa6, 0xa6, 0xa6, 0xa6, 0xa6]; // from RFC3394 Section 2.2.3.1
pub const IV_5649: [u8; 4] = [0xa6, 0x59, 0x59, 0xa6]; // from RFC5649 Section 3

// See the AES Key Wrap definition RFC and update
// * RFC3394 "Advanced Encryption Standard (AES) Key Wrap Algorithm"
//   https://tools.ietf.org/html/rfc3394.html
//   (algorithm outlined in comments below)
// * RFC 5649 "Advanced Encryption Standard (AES) Key Wrap with Padding Algorithm"
//   https://tools.ietf.org/html/rfc5649.html
//   (algorithm not repeated here, relatively minor additions)

#[inline(always)]
fn to_u8_8_array(array: &[u8]) -> [u8; 8] {
    unsafe { *(array as *const _ as *const _) }
}

#[inline(always)]
fn to_u8_4_array(array: &[u8]) -> [u8; 4] {
    unsafe { *(array as *const _ as *const _) }
}

#[inline(always)]
fn u32_from_be_u8(buffer: &[u8; 4]) -> u32 {
    u32::from(buffer[3])
        | u32::from(buffer[2]) << 8
        | u32::from(buffer[1]) << 16
        | u32::from(buffer[0]) << 24
}

#[inline(always)]
fn u64_from_be_u8(buffer: &[u8; 8]) -> u64 {
    u64::from(buffer[7])
        | u64::from(buffer[6]) << 8
        | u64::from(buffer[5]) << 16
        | u64::from(buffer[4]) << 24
        | u64::from(buffer[3]) << 32
        | u64::from(buffer[2]) << 40
        | u64::from(buffer[1]) << 48
        | u64::from(buffer[0]) << 56
}

#[inline(always)]
pub fn u64_as_u8(src: &[u64]) -> &[u8] {
    unsafe { core::slice::from_raw_parts(src.as_ptr() as *const u8, src.len() * 8) }
}

#[inline(always)]
pub fn u32_as_u8(src: &[u32]) -> &[u8] {
    unsafe { core::slice::from_raw_parts(src.as_ptr() as *const u8, src.len() * 8) }
}

/// key wrapping as defined in RFC 3394
/// http://www.ietf.org/rfc/rfc3394.txt
pub fn aes_unwrap_key(kek: &[u8], wrapped: &[u8]) -> Vec<u8> {
    let (key, key_iv) = aes_unwrap_key_and_iv(kek, wrapped);
    if key_iv != IV_3394 {
        panic!("{}", "ERROR");
    }

    return key;
}

pub fn aes_unwrap_key_and_iv(kek: &[u8], wrapped: &[u8]) -> (Vec<u8>, Vec<u8>) {
    match kek.len() {
        16 => Aes128Kw::aes_unwrap_key_and_iv(kek, wrapped),
        24 => Aes192Kw::aes_unwrap_key_and_iv(kek, wrapped),
        32 => Aes256Kw::aes_unwrap_key_and_iv(kek, wrapped),
        _ => panic!("kek is not supported: {:?}", kek),
    }
}

/// alternate initial value for aes key wrapping, as defined in RFC 5649 section 3
/// http://www.ietf.org/rfc/rfc5649.txt
#[inline]
pub fn aes_unwrap_key_with_pad(kek: &[u8], wrapped: &[u8]) -> Vec<u8> {
    match kek.len() {
        16 => Aes128Kw::aes_unwrap_key_with_pad(kek, wrapped),
        24 => Aes192Kw::aes_unwrap_key_with_pad(kek, wrapped),
        32 => Aes256Kw::aes_unwrap_key_with_pad(kek, wrapped),
        _ => panic!("kek is not supported: {:?}", kek),
    }
}

pub fn aes_wrap_key_and_iv(kek: &[u8], plaintext: &[u8], iv: &[u8]) -> Vec<u8> {
    match kek.len() {
        16 => Aes128Kw::aes_wrap_key_and_iv(kek, plaintext, iv),
        24 => Aes192Kw::aes_wrap_key_and_iv(kek, plaintext, iv),
        32 => Aes256Kw::aes_wrap_key_and_iv(kek, plaintext, iv),
        _ => panic!("kek is not supported: {:?}", kek),
    }
}

pub fn aes_wrap_key(kek: &[u8], plaintext: &[u8]) -> Vec<u8> {
    return aes_wrap_key_and_iv(kek, plaintext, &IV_3394);
}

pub fn aes_wrap_key_with_pad(kek: &[u8], plaintext: &[u8]) -> Vec<u8> {
    match kek.len() {
        16 => Aes128Kw::aes_wrap_key_with_pad(kek, plaintext),
        24 => Aes192Kw::aes_wrap_key_with_pad(kek, plaintext),
        32 => Aes256Kw::aes_wrap_key_with_pad(kek, plaintext),
        _ => panic!("kek is not supported: {:?}", kek),
    }
}

macro_rules! impl_aes_keywrap {
    ($name: tt, $cipher:tt) => {
        pub struct $name {}

        impl $name {
            pub fn aes_unwrap_key_and_iv(kek: &[u8], wrapped: &[u8]) -> (Vec<u8>, Vec<u8>) {
                let n = wrapped.len() / 8 - 1;
                let mut r: Vec<[u8; 8]> = Vec::new();
                r.push([0u8; 8]);
                for i in 1..n + 1 {
                    r.push(to_u8_8_array(&wrapped[i * 8..i * 8 + 8]));
                }

                let mut a = u64_from_be_u8(&to_u8_8_array(&wrapped[..8]));
                let mut cipher = $cipher::new(kek);

                for j in (0..6).rev() {
                    for i in (1..n + 1).rev() {
                        let mut ciphertext: Vec<u8> = Vec::new();
                        ciphertext
                            .write(&(a ^ (n * j + i) as u64).to_be_bytes())
                            .unwrap();
                        ciphertext.write(&r[i]).unwrap();
                        cipher.decrypt(&mut ciphertext);
                        a = u64_from_be_u8(&to_u8_8_array(&ciphertext[..8]));
                        r[i].copy_from_slice(&ciphertext[8..]);
                    }
                }

                let mut key: Vec<u8> = Vec::new();
                for v in &r[1..] {
                    key.write(v).unwrap();
                }

                (key, a.to_be_bytes().to_vec())
            }

            pub fn aes_wrap_key_and_iv(kek: &[u8], plaintext: &[u8], iv: &[u8]) -> Vec<u8> {
                let n = plaintext.len() / 8;
                let mut r: Vec<[u8; 8]> = Vec::new();
                r.push([0u8; 8]);
                for i in 0..n {
                    r.push(to_u8_8_array(&plaintext[i * 8..i * 8 + 8]));
                }

                let mut a = u64_from_be_u8(&to_u8_8_array(&iv[..8]));
                let mut cipher = $cipher::new(kek);

                for j in 0..6 {
                    for i in 1..n + 1 {
                        let mut ciphertext: Vec<u8> = Vec::new();
                        ciphertext.write(&a.to_be_bytes()).unwrap();
                        ciphertext.write(&r[i]).unwrap();
                        cipher.encrypt(&mut ciphertext);
                        a = u64_from_be_u8(&to_u8_8_array(&ciphertext[..8])) ^ (n * j + i) as u64;
                        r[i].copy_from_slice(&ciphertext[8..]);
                    }
                }

                let mut ret: Vec<u8> = Vec::new();
                ret.write(&a.to_be_bytes()).unwrap();
                for v in &r[1..] {
                    ret.write(v).unwrap();
                }

                ret
            }

            pub fn aes_unwrap_key_with_pad(kek: &[u8], wrapped: &[u8]) -> Vec<u8> {
                let mut key: Vec<u8> = Vec::new();
                let mut key_iv: Vec<u8> = Vec::new();
                if wrapped.len() == 16 {
                    let mut cipher = $cipher::new(kek);
                    let mut plaintext: Vec<u8> = Vec::new();
                    plaintext.write(wrapped).unwrap();
                    cipher.decrypt(&mut plaintext);
                    key_iv.write(&plaintext[..8]).unwrap();
                    key.write(&plaintext[8..]).unwrap();
                } else {
                    let (_key, _key_iv) = aes_unwrap_key_and_iv(kek, wrapped);
                    key.write(&_key).unwrap();
                    key_iv.write(&_key_iv).unwrap();
                }

                if IV_5649 != to_u8_4_array(&key_iv[..4]) {
                    panic!(
                        "Integrity Check Failed: {:?} (expected A65959A6)",
                        to_u8_4_array(&key_iv[..4])
                    );
                }

                //RFC5649: 32bit fixed + 32bit length
                let key_len: usize = u32_from_be_u8(&to_u8_4_array(&key_iv[4..])) as usize;
                return key[..key_len].to_vec();
            }

            pub fn aes_wrap_key_with_pad(kek: &[u8], plaintext: &[u8]) -> Vec<u8> {
                let mut iv: Vec<u8> = Vec::new();
                //RFC5649: 32bit fixed + 32bit length
                iv.write(&IV_5649).unwrap();
                iv.write(&(plaintext.len() as u32).to_be_bytes()).unwrap();

                let mut pad_pt: Vec<u8> = Vec::new();
                pad_pt.write(plaintext).unwrap();
                let n = ((8 - plaintext.len() as i32) % 8).abs() as usize;
                for _ in 0..n {
                    pad_pt.push(0u8);
                }

                if pad_pt.len() == 8 {
                    let mut cipher = $cipher::new(kek);
                    let mut wrapped: Vec<u8> = Vec::new();
                    wrapped.write(&iv).unwrap();
                    wrapped.write(&pad_pt).unwrap();
                    cipher.encrypt(&mut wrapped);
                    return wrapped.to_vec();
                }

                return aes_wrap_key_and_iv(kek, &pad_pt, &iv);
            }
        }
    };
}

impl_aes_keywrap!(Aes128Kw, Aes128Ecb);
impl_aes_keywrap!(Aes192Kw, Aes192Ecb);
impl_aes_keywrap!(Aes256Kw, Aes256Ecb);

#[cfg(test)]
mod tests {
    use super::*;
    use hex;
    use test::Bencher;

    // RFC3394 tests
    #[test]
    fn test_128bit_kek_and_128bit_key() {
        let kek = hex::decode("000102030405060708090A0B0C0D0E0F").unwrap();
        let cipher = hex::decode("1FA68B0A8112B447AEF34BD8FB5A7B829D3E862371D2CFE5").unwrap();
        let plain = hex::decode("00112233445566778899AABBCCDDEEFF").unwrap();
        assert_eq!(cipher, aes_wrap_key(&kek, &plain));
        assert_eq!(plain, aes_unwrap_key(&kek, &cipher));
    }

    #[test]
    fn test_192bit_kek_and_128bit_key() {
        let kek = hex::decode("000102030405060708090A0B0C0D0E0F1011121314151617").unwrap();
        let cipher = hex::decode("96778B25AE6CA435F92B5B97C050AED2468AB8A17AD84E5D").unwrap();
        let plain = hex::decode("00112233445566778899AABBCCDDEEFF").unwrap();
        assert_eq!(cipher, aes_wrap_key(&kek, &plain));
        assert_eq!(plain, aes_unwrap_key(&kek, &cipher));
    }

    #[test]
    fn test_256bit_kek_and_128bit_key() {
        let kek = hex::decode("000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F")
            .unwrap();
        let cipher = hex::decode("64E8C3F9CE0F5BA263E9777905818A2A93C8191E7D6E8AE7").unwrap();
        let plain = hex::decode("00112233445566778899AABBCCDDEEFF").unwrap();
        assert_eq!(cipher, aes_wrap_key(&kek, &plain));
        assert_eq!(plain, aes_unwrap_key(&kek, &cipher));
    }

    #[test]
    fn test_192bit_kek_and_192bit_key() {
        let kek = hex::decode("000102030405060708090A0B0C0D0E0F1011121314151617").unwrap();
        let cipher =
            hex::decode("031D33264E15D33268F24EC260743EDCE1C6C7DDEE725A936BA814915C6762D2")
                .unwrap();
        let plain = hex::decode("00112233445566778899AABBCCDDEEFF0001020304050607").unwrap();
        assert_eq!(cipher, aes_wrap_key(&kek, &plain));
        assert_eq!(plain, aes_unwrap_key(&kek, &cipher));
    }

    #[test]
    fn test_256bit_kek_and_192bit_key() {
        let kek = hex::decode("000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F")
            .unwrap();
        let cipher =
            hex::decode("A8F9BC1612C68B3FF6E6F4FBE30E71E4769C8B80A32CB8958CD5D17D6B254DA1")
                .unwrap();
        let plain = hex::decode("00112233445566778899AABBCCDDEEFF0001020304050607").unwrap();
        assert_eq!(cipher, aes_wrap_key(&kek, &plain));
        assert_eq!(plain, aes_unwrap_key(&kek, &cipher));
    }

    #[test]
    fn test_256bit_kek_and_256bit_key() {
        let kek = hex::decode("000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F")
            .unwrap();
        let cipher = hex::decode(
            "28C9F404C4B810F4CBCCB35CFB87F8263F5786E2D80ED326CBC7F0E71A99F43BFB988B9B7A02DD21",
        )
        .unwrap();
        let plain = hex::decode("00112233445566778899AABBCCDDEEFF000102030405060708090A0B0C0D0E0F")
            .unwrap();
        assert_eq!(cipher, aes_wrap_key(&kek, &plain));
        assert_eq!(plain, aes_unwrap_key(&kek, &cipher));
    }

    // RFC5649 tests
    #[test]
    fn test_padded_192bit_kek_and_20_octets_key() {
        let kek = hex::decode("5840DF6E29B02AF1AB493B705BF16EA1AE8338F4DCC176A8").unwrap();
        let cipher =
            hex::decode("138BDEAA9B8FA7FC61F97742E72248EE5AE6AE5360D1AE6A5F54F373FA543B6A")
                .unwrap();
        let plain = hex::decode("C37B7E6492584340BED12207808941155068F738").unwrap();
        assert_eq!(cipher, aes_wrap_key_with_pad(&kek, &plain));
        assert_eq!(plain, aes_unwrap_key_with_pad(&kek, &cipher));
    }

    #[test]
    fn test_padded_192bit_kek_and_7_octets_key() {
        let kek = hex::decode("5840DF6E29B02AF1AB493B705BF16EA1AE8338F4DCC176A8").unwrap();
        let cipher = hex::decode("AFBEB0F07DFBF5419200F2CCB50BB24F").unwrap();
        let plain = hex::decode("466F7250617369").unwrap();
        assert_eq!(cipher, aes_wrap_key_with_pad(&kek, &plain));
        assert_eq!(plain, aes_unwrap_key_with_pad(&kek, &cipher));
    }

    #[bench]
    fn bench_128bit_key_wrap(b: &mut Bencher) {
        b.iter(|| {
            for _ in 0..100 {
                let kek = hex::decode("000102030405060708090A0B0C0D0E0F").unwrap();
                let plain = hex::decode("00112233445566778899AABBCCDDEEFF").unwrap();
                aes_wrap_key(&kek, &plain);
            }
        });
    }

    #[bench]
    fn bench_128bit_key_unwrap(b: &mut Bencher) {
        b.iter(|| {
            for _ in 0..100 {
                let kek = hex::decode("000102030405060708090A0B0C0D0E0F").unwrap();
                let cipher =
                    hex::decode("1FA68B0A8112B447AEF34BD8FB5A7B829D3E862371D2CFE5").unwrap();
                aes_unwrap_key(&kek, &cipher);
            }
        });
    }

    #[bench]
    fn bench_15628bit_key_wrap(b: &mut Bencher) {
        b.iter(|| {
            for _ in 0..100 {
                let kek =
                    hex::decode("000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F")
                        .unwrap();
                let plain = hex::decode("00112233445566778899AABBCCDDEEFF").unwrap();
                aes_wrap_key(&kek, &plain);
            }
        });
    }

    #[bench]
    fn bench_256bit_key_unwrap(b: &mut Bencher) {
        b.iter(|| {
            for _ in 0..100 {
                let kek =
                    hex::decode("000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F")
                        .unwrap();
                let cipher =
                    hex::decode("64E8C3F9CE0F5BA263E9777905818A2A93C8191E7D6E8AE7").unwrap();
                aes_unwrap_key(&kek, &cipher);
            }
        });
    }
}
