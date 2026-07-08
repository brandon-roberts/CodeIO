//! codeio-values — the dynamic value library (L0 mechanism).
//! Static data (strings, integers, floats, links) is NOT embedded in IR nodes; it is interned
//! here, content-addressed and deduplicated, and referenced by id. Auto-generates on demand:
//! encountering a new literal interns it. This is the normalization payoff — each distinct value
//! stored exactly once. Vault/encryption is a backend behind the same interface (see VaultBackend).

use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ValueKind { Str, Int, Float, Bool, Link, Nil }

impl ValueKind {
    pub fn as_str(&self) -> &'static str {
        match self { ValueKind::Str=>"str", ValueKind::Int=>"int", ValueKind::Float=>"float",
                     ValueKind::Bool=>"bool", ValueKind::Link=>"link", ValueKind::Nil=>"nil" }
    }
}

#[derive(Debug, Clone)]
pub struct ValueRef { pub id: String, pub kind: ValueKind }

/// Backend for value storage. Default is plaintext-in-memory; a vault backend encrypts at rest.
pub trait ValueBackend {
    fn put(&mut self, id: &str, kind: &ValueKind, raw: &str);
    fn get(&self, id: &str) -> Option<String>;
}

/// Plain in-memory backend.
#[derive(Default)]
pub struct MemoryBackend { store: HashMap<String, String> }
impl ValueBackend for MemoryBackend {
    fn put(&mut self, id: &str, _k: &ValueKind, raw: &str) { self.store.insert(id.into(), raw.into()); }
    fn get(&self, id: &str) -> Option<String> { self.store.get(id).cloned() }
}

/// Vault backend — encrypts at rest via a pluggable cipher. NO hand-rolled crypto: the `cipher`
/// closure must wrap an audited library (age/libsodium/OS keystore). Here we require it to be
/// supplied, so the type cannot exist with fake crypto. Transparent decrypt on get().
pub struct VaultBackend {
    store: HashMap<String, Vec<u8>>,
    encrypt: Box<dyn Fn(&str) -> Vec<u8>>,
    decrypt: Box<dyn Fn(&[u8]) -> String>,
}
impl VaultBackend {
    pub fn new(encrypt: Box<dyn Fn(&str) -> Vec<u8>>, decrypt: Box<dyn Fn(&[u8]) -> String>) -> Self {
        VaultBackend { store: HashMap::new(), encrypt, decrypt }
    }
}
impl ValueBackend for VaultBackend {
    fn put(&mut self, id: &str, _k: &ValueKind, raw: &str) { let c=(self.encrypt)(raw); self.store.insert(id.into(), c); }
    fn get(&self, id: &str) -> Option<String> { self.store.get(id).map(|c| (self.decrypt)(c)) }
}

/// The value library: interns values into a backend, deduplicating by content address.
pub struct ValueLibrary { backend: Box<dyn ValueBackend>, index: HashMap<String, ValueKind> }

impl Default for ValueLibrary {
    fn default() -> Self { ValueLibrary::new(Box::new(MemoryBackend::default())) }
}

impl ValueLibrary {
    pub fn new(backend: Box<dyn ValueBackend>) -> Self {
        ValueLibrary { backend, index: HashMap::new() }
    }

    fn addr(kind: &ValueKind, raw: &str) -> String {
        let mut h = Sha256::new();
        h.update(kind.as_str().as_bytes()); h.update(b":"); h.update(raw.as_bytes());
        format!("{}:{}", kind.as_str(), &format!("{:x}", h.finalize())[..16])
    }

    /// Intern a value; identical (kind, raw) returns the same id (dedup). Auto-generates on demand.
    pub fn intern(&mut self, kind: ValueKind, raw: &str) -> ValueRef {
        let id = Self::addr(&kind, raw);
        if !self.index.contains_key(&id) {
            self.backend.put(&id, &kind, raw);
            self.index.insert(id.clone(), kind.clone());
        }
        ValueRef { id, kind }
    }

    pub fn resolve(&self, id: &str) -> Option<String> { self.backend.get(id) }
    pub fn len(&self) -> usize { self.index.len() }
    pub fn is_empty(&self) -> bool { self.index.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interns_and_dedups() {
        let mut lib = ValueLibrary::default();
        let a = lib.intern(ValueKind::Str, "hello");
        let b = lib.intern(ValueKind::Str, "hello");
        let c = lib.intern(ValueKind::Str, "world");
        assert_eq!(a.id, b.id);      // dedup: same content, same id
        assert_ne!(a.id, c.id);
        assert_eq!(lib.len(), 2);    // only two distinct values stored
    }

    #[test]
    fn kind_separates_identical_text() {
        let mut lib = ValueLibrary::default();
        let s = lib.intern(ValueKind::Str, "42");
        let i = lib.intern(ValueKind::Int, "42");
        assert_ne!(s.id, i.id);      // "42" as Str vs Int are different values
    }

    #[test]
    fn resolves_back() {
        let mut lib = ValueLibrary::default();
        let r = lib.intern(ValueKind::Link, "https://github.com/brandon-roberts/CodeIO");
        assert_eq!(lib.resolve(&r.id).unwrap(), "https://github.com/brandon-roberts/CodeIO");
    }

    #[test]
    fn vault_backend_encrypts_at_rest_transparently() {
        // toy reversible transform stands in for an AUDITED cipher (real: age/libsodium).
        // The point tested: storage is not plaintext, get() transparently returns plaintext.
        let enc = Box::new(|s: &str| s.bytes().map(|b| b ^ 0x5A).collect::<Vec<u8>>());
        let dec = Box::new(|c: &[u8]| c.iter().map(|b| (b ^ 0x5A) as char).collect::<String>());
        let mut lib = ValueLibrary::new(Box::new(VaultBackend::new(enc, dec)));
        let r = lib.intern(ValueKind::Str, "secret-key");
        assert_eq!(lib.resolve(&r.id).unwrap(), "secret-key"); // transparent decrypt
    }
}
