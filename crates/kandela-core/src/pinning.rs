//! Certificate pinning for the bridge connection.
//!
//! A bridge's certificate is issued by Signify's private CA for the bridge
//! id, not for its IP address, so normal verification can't work. Instead
//! the app remembers the SHA-256 of the certificate it saw while pairing
//! ("trust on first use") and refuses any other certificate afterwards, so
//! the application key is never sent to a device impersonating the bridge.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Once};

use rustls::DigitallySignedStruct;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, ring};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use sha2::{Digest, Sha256};

/// SHA-256 of a certificate's DER encoding.
pub type Fingerprint = [u8; 32];

pub fn fingerprint(cert: &[u8]) -> Fingerprint {
    Sha256::digest(cert).into()
}

pub fn to_hex(fp: &Fingerprint) -> String {
    fp.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn from_hex(s: &str) -> Option<Fingerprint> {
    let s = s.trim();
    if s.len() != 64 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Makes ring the process-wide TLS crypto provider, which reqwest needs for
/// ordinary (CA-verified) connections such as cloud discovery.
pub fn install_crypto_provider() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = ring::default_provider().install_default();
    });
}

/// Accepts exactly one certificate, or, without a pin, records whichever
/// certificate the server presents.
#[derive(Debug)]
pub struct PinnedVerifier {
    pin: Option<Fingerprint>,
    seen: Mutex<Option<Fingerprint>>,
    mismatch: AtomicBool,
    algorithms: WebPkiSupportedAlgorithms,
}

impl PinnedVerifier {
    /// Only accepts the certificate with this fingerprint.
    pub fn pinned(pin: Fingerprint) -> Arc<Self> {
        Self::new(Some(pin))
    }

    /// Accepts any certificate and records it, for first contact with a
    /// bridge. Never send secrets over such a connection.
    pub fn recording() -> Arc<Self> {
        Self::new(None)
    }

    fn new(pin: Option<Fingerprint>) -> Arc<Self> {
        Arc::new(Self {
            pin,
            seen: Mutex::new(None),
            mismatch: AtomicBool::new(false),
            algorithms: ring::default_provider().signature_verification_algorithms,
        })
    }

    /// The fingerprint of the last certificate the server presented.
    pub fn seen(&self) -> Option<Fingerprint> {
        *self.seen.lock().unwrap()
    }

    /// Whether a connection was refused because the certificate didn't match.
    pub fn mismatched(&self) -> bool {
        self.mismatch.load(Ordering::SeqCst)
    }

    /// A rustls configuration that verifies servers with this verifier.
    pub fn client_config(self: &Arc<Self>) -> rustls::ClientConfig {
        rustls::ClientConfig::builder_with_provider(Arc::new(ring::default_provider()))
            .with_safe_default_protocol_versions()
            .expect("ring supports the default TLS versions")
            .dangerous()
            .with_custom_certificate_verifier(self.clone())
            .with_no_client_auth()
    }
}

impl ServerCertVerifier for PinnedVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let fp = fingerprint(end_entity);
        *self.seen.lock().unwrap() = Some(fp);
        match self.pin {
            Some(pin) if pin != fp => {
                self.mismatch.store(true, Ordering::SeqCst);
                Err(rustls::Error::General(
                    "bridge certificate does not match the pinned one".into(),
                ))
            }
            _ => Ok(ServerCertVerified::assertion()),
        }
    }

    // The handshake signatures are still checked, so the server must hold the
    // private key of the certificate it presents.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(v: &PinnedVerifier, cert: &[u8]) -> bool {
        let name = ServerName::try_from("192.168.1.100").unwrap();
        v.verify_server_cert(
            &CertificateDer::from(cert),
            &[],
            &name,
            &[],
            UnixTime::now(),
        )
        .is_ok()
    }

    #[test]
    fn hex_round_trip() {
        let fp = fingerprint(b"certificate");
        assert_eq!(from_hex(&to_hex(&fp)), Some(fp));
        assert_eq!(from_hex("abc"), None);
        assert_eq!(from_hex(&"zz".repeat(32)), None);
    }

    #[test]
    fn recording_accepts_and_remembers() {
        let v = PinnedVerifier::recording();
        assert!(check(&v, b"cert A"));
        assert_eq!(v.seen(), Some(fingerprint(b"cert A")));
        assert!(!v.mismatched());
    }

    #[test]
    fn pinned_accepts_only_its_certificate() {
        let v = PinnedVerifier::pinned(fingerprint(b"cert A"));
        assert!(check(&v, b"cert A"));
        assert!(!v.mismatched());
        assert!(!check(&v, b"cert B"));
        assert!(v.mismatched());
    }
}
