//! Time-based one-time passwords (RFC 6238), for 2FA items.
//!
//! An item's value is either a base32 secret key (what sites show under
//! "can't scan the QR code?") or a full `otpauth://totp/...` link (what the QR
//! code itself contains).

use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algorithm {
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub secret: Vec<u8>,
    pub digits: u32,
    pub period: u64,
    pub algorithm: Algorithm,
}

/// Parses a base32 key or an `otpauth://totp/` link.
pub fn parse(value: &str) -> Result<Config, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("Enter the secret key.".into());
    }
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("otpauth-migration://") {
        return Err("Google Authenticator export codes aren't supported. Use the site's own 2FA setup QR or key.".into());
    }
    if lower.starts_with("otpauth://") {
        return parse_uri(value);
    }
    Ok(Config {
        secret: decode_base32(value)?,
        digits: 6,
        period: 30,
        algorithm: Algorithm::Sha1,
    })
}

fn parse_uri(uri: &str) -> Result<Config, String> {
    let rest = &uri["otpauth://".len()..];
    if !rest.to_ascii_lowercase().starts_with("totp/") {
        return Err("Only time-based (TOTP) 2FA codes are supported.".into());
    }
    let query = rest.split_once('?').map(|(_, q)| q).unwrap_or("");
    let mut secret = None;
    let mut digits = 6;
    let mut period = 30;
    let mut algorithm = Algorithm::Sha1;
    for pair in query.split('&') {
        let (key, val) = pair.split_once('=').unwrap_or((pair, ""));
        let val = percent_decode(val);
        match key.to_ascii_lowercase().as_str() {
            "secret" => secret = Some(decode_base32(&val)?),
            "digits" => {
                digits = val.parse().map_err(|_| "Invalid digits in 2FA link.".to_string())?;
                if !(6..=8).contains(&digits) {
                    return Err("2FA codes must be 6 to 8 digits.".into());
                }
            }
            "period" => {
                period = val.parse().map_err(|_| "Invalid period in 2FA link.".to_string())?;
                if period == 0 {
                    return Err("Invalid period in 2FA link.".into());
                }
            }
            "algorithm" => {
                algorithm = match val.to_ascii_uppercase().as_str() {
                    "SHA1" => Algorithm::Sha1,
                    "SHA256" => Algorithm::Sha256,
                    "SHA512" => Algorithm::Sha512,
                    other => return Err(format!("Unsupported 2FA algorithm {other}.")),
                }
            }
            _ => {}
        }
    }
    Ok(Config {
        secret: secret.ok_or("The 2FA link has no secret.")?,
        digits,
        period,
        algorithm,
    })
}

/// RFC 4648 base32, ignoring case, spaces, dashes and `=` padding.
fn decode_base32(input: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let (mut buffer, mut bits) = (0u64, 0u32);
    let mut count = 0;
    for c in input.chars() {
        if c.is_whitespace() || c == '-' || c == '=' {
            continue;
        }
        let v = match c.to_ascii_uppercase() {
            ch @ 'A'..='Z' => ch as u64 - 'A' as u64,
            ch @ '2'..='7' => ch as u64 - '2' as u64 + 26,
            _ => return Err("The secret key can only contain letters A–Z and digits 2–7.".into()),
        };
        buffer = (buffer << 5) | v;
        bits += 5;
        count += 1;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    if count < 16 {
        return Err("That secret key is too short.".into());
    }
    Ok(out)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = std::str::from_utf8(&bytes[i + 1..i + 3])
                .map_err(|_| ())
                .and_then(|h| u8::from_str_radix(h, 16).map_err(|_| ()))
            {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hmac(algorithm: Algorithm, key: &[u8], msg: &[u8]) -> Vec<u8> {
    fn run<M: Mac + hmac::digest::KeyInit>(key: &[u8], msg: &[u8]) -> Vec<u8> {
        let mut mac = <M as hmac::digest::KeyInit>::new_from_slice(key).expect("HMAC takes any key length");
        mac.update(msg);
        mac.finalize().into_bytes().to_vec()
    }
    match algorithm {
        Algorithm::Sha1 => run::<Hmac<sha1::Sha1>>(key, msg),
        Algorithm::Sha256 => run::<Hmac<sha2::Sha256>>(key, msg),
        Algorithm::Sha512 => run::<Hmac<sha2::Sha512>>(key, msg),
    }
}

pub fn code_at(cfg: &Config, unix_secs: u64) -> String {
    let counter = unix_secs / cfg.period;
    let hash = hmac(cfg.algorithm, &cfg.secret, &counter.to_be_bytes());
    let offset = (hash[hash.len() - 1] & 0x0f) as usize;
    let binary = u32::from_be_bytes([hash[offset], hash[offset + 1], hash[offset + 2], hash[offset + 3]])
        & 0x7fff_ffff;
    let code = binary % 10u32.pow(cfg.digits);
    format!("{code:0width$}", width = cfg.digits as usize)
}

/// The current code, how many seconds it stays valid, and the code period.
pub fn now(value: &str) -> Result<(String, u64, u64), String> {
    let cfg = parse(value)?;
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    Ok((code_at(&cfg, secs), cfg.period - secs % cfg.period, cfg.period))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rfc(algorithm: Algorithm, secret: &[u8]) -> Config {
        Config { secret: secret.to_vec(), digits: 8, period: 30, algorithm }
    }

    #[test]
    fn rfc6238_vectors() {
        let sha1 = rfc(Algorithm::Sha1, b"12345678901234567890");
        let sha256 = rfc(Algorithm::Sha256, b"12345678901234567890123456789012");
        let sha512 = rfc(
            Algorithm::Sha512,
            b"1234567890123456789012345678901234567890123456789012345678901234",
        );
        let cases = [
            (59, "94287082", "46119246", "90693936"),
            (1111111109, "07081804", "68084774", "25091201"),
            (1111111111, "14050471", "67062674", "99943326"),
            (1234567890, "89005924", "91819424", "93441116"),
            (2000000000, "69279037", "90698825", "38618901"),
            (20000000000, "65353130", "77737706", "47863826"),
        ];
        for (t, a, b, c) in cases {
            assert_eq!(code_at(&sha1, t), a);
            assert_eq!(code_at(&sha256, t), b);
            assert_eq!(code_at(&sha512, t), c);
        }
    }

    #[test]
    fn parses_base32_key() {
        let cfg = parse("6GI6 RG3J WY54 ESN2 EL5U 3HAZ WH6T VQBS BLRS 64FZ 7DRK 5Y7V 5HQU 7KYZ").unwrap();
        assert_eq!(cfg.secret.len(), 35);
        assert_eq!(cfg.digits, 6);
        assert_eq!(code_at(&cfg, 0).len(), 6);
        // "12345678901234567890" in base32, lower case.
        let rfc_key = parse("gezdgnbvgy3tqojqgezdgnbvgy3tqojq").unwrap();
        assert_eq!(rfc_key.secret, b"12345678901234567890");
    }

    #[test]
    fn parses_otpauth_link() {
        let cfg = parse(
            "otpauth://totp/GitHub:me%40x.com?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&issuer=GitHub&digits=8&period=60&algorithm=SHA256",
        )
        .unwrap();
        assert_eq!(cfg.secret, b"12345678901234567890");
        assert_eq!((cfg.digits, cfg.period, cfg.algorithm), (8, 60, Algorithm::Sha256));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse("").is_err());
        assert!(parse("not-a-key!").is_err());
        assert!(parse("ABCD").is_err());
        assert!(parse("otpauth://hotp/x?secret=GEZDGNBVGY3TQOJQ").is_err());
        assert!(parse("otpauth://totp/x?issuer=a").is_err());
        assert!(parse("otpauth-migration://offline?data=abc").is_err());
    }
}
