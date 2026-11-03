//! Snapshot files in Rust: the snapshot container (`DJSN`, D2260; layout in docs/host.md
//! "Snapshots", written by js/src/snapshot.ts), session images and raw heap images
//! (`Vm::bytes`), read and written the way the JS host does (D2264). There is one container
//! format; any other version is refused (D024).
//!
//! A container holds a session image (D2066), as a zstd frame compressed with a *reference
//! image* as a raw-content dictionary: this build's base snapshot, or pi's prelude image,
//! which the reader has to be given. Its header names what the image depends on (D2265):
//! the image's dependency descriptor and the hash of its base's builtin prefix. A
//! container reads into any djs with the same ones.

use djs_core::heap::CellKind;
use djs_core::heap::cells::{MAGIC, Roots};
use djs_core::vm::{IMAGE_DEPS, ImageDep, ImageDeps, session};

/// "DJSN", little-endian.
pub const CONTAINER_MAGIC: u32 = 0x4e53_4a44;
/// The container's version (js/src/snapshot.ts `SNAPSHOT_FORMAT`).
pub const FORMAT: u16 = 3;
const HEADER: usize = 68;

/// The app byte: what the metadata is.
pub const APP_SANDBOX: u8 = 1;
pub const APP_PI_SESSION: u8 = 2;
/// The djs REPL's `.snapshot` (D2340): metadata `{}`.
pub const APP_REPL: u8 = 3;

pub const REFERENCE_NONE: u8 = 0;
/// This build's base snapshot (`djs_prelude::BASE`).
pub const REFERENCE_BASE: u8 = 1;
/// pi's prelude image: the session's VM right after pi's prelude, sealed.
pub const REFERENCE_PI_PRELUDE: u8 = 2;

/// What the session image's prefix words are against (byte 41): the build's base image,
/// or the container's reference.
pub const AGAINST_BASE: u8 = 0;
pub const AGAINST_REFERENCE: u8 = 1;

pub const COMPRESSION_NONE: u8 = 0;
pub const COMPRESSION_ZSTD: u8 = 1;

/// js/src/snapshot.ts's `DEFAULT_ZSTD_LEVEL` (D2261).
pub const DEFAULT_LEVEL: i32 = 3;

/// The header fields of a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub app: u8,
    pub compression: u8,
    /// The image's dependency descriptor (`vm::ImageDeps`, D2265).
    pub deps: ImageDeps,
    /// FNV-1a 64 of its base's builtin prefix (`session::prefix_hash`).
    pub prefix: u64,
    pub reference_kind: u8,
    /// FNV-1a 64 of the reference image.
    pub reference_hash: u64,
    pub reference_len: u32,
    pub image_len: u32,
    pub payload_len: u32,
    /// [`AGAINST_BASE`] or [`AGAINST_REFERENCE`].
    pub against: u8,
}

/// How a snapshot file was stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    RawImage,
    /// A session image (D2066) on its own.
    SessionImage,
    Container,
}

/// A snapshot file, unwrapped.
#[derive(Debug)]
pub struct Snapshot {
    pub format: Format,
    pub header: Option<Header>,
    /// The host's metadata (JSON text): the Sandbox's host function names and clock, the
    /// pi adapter's session state.
    pub meta: Option<String>,
    pub image: Vec<u8>,
    /// The reference a compressed container was decompressed with.
    pub reference: Option<Vec<u8>>,
}

impl Header {
    pub fn app_name(&self) -> String {
        match self.app {
            APP_SANDBOX => "sandbox".into(),
            APP_PI_SESSION => "pi session".into(),
            APP_REPL => "repl".into(),
            n => format!("app {n}"),
        }
    }
}

impl Snapshot {
    /// "raw image", "DJSN container (pi session, zstd against pi's prelude image)", ...
    pub fn describe(&self) -> String {
        match (self.format, &self.header) {
            (Format::RawImage, _) => "raw image".into(),
            (Format::SessionImage, _) => "session image".into(),
            (Format::Container, Some(h)) => format!(
                "DJSN container ({}, {})",
                h.app_name(),
                if h.compression == COMPRESSION_ZSTD {
                    format!("zstd against {}", reference_name(h.reference_kind))
                } else {
                    "uncompressed".into()
                }
            ),
            (Format::Container, None) => "DJSN container".into(),
        }
    }
}

pub fn reference_name(kind: u8) -> &'static str {
    match kind {
        REFERENCE_NONE => "no reference",
        REFERENCE_BASE => "the base image",
        REFERENCE_PI_PRELUDE => "pi's prelude image",
        _ => "an unknown reference",
    }
}

/// FNV-1a 64: the container's reference hash (js/src/snapshot.ts `fnv1a64`).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Whether `bytes` start like a heap image: the roots cell with the magic.
pub fn is_image(bytes: &[u8]) -> bool {
    bytes.first() == Some(&(CellKind::Roots as u8))
        && u32_at(bytes, Roots::MAGIC.offset as usize) == Some(MAGIC)
}

/// Reads any snapshot file. `reference` is the reference image of a compressed container
/// (a raw image or an uncompressed container of one); without one, a container against
/// the base image uses this build's base if its hash matches.
pub fn read(bytes: &[u8], reference: Option<&[u8]>) -> Result<Snapshot, String> {
    if is_image(bytes) {
        return Ok(Snapshot {
            format: Format::RawImage,
            header: None,
            meta: None,
            image: bytes.to_vec(),
            reference: None,
        });
    }
    if session::is_session_image(bytes) {
        let base = match reference {
            Some(r) => image_of(r)?,
            None => djs_prelude::BASE.to_vec(),
        };
        let image = expand(bytes, &base)?;
        return Ok(Snapshot {
            format: Format::SessionImage,
            header: None,
            meta: None,
            image,
            reference: None,
        });
    }
    if u32_at(bytes, 0) != Some(CONTAINER_MAGIC) {
        return Err("not a heap image or snapshot (no image roots, no DJSs/DJSN magic)".into());
    }
    let (h, meta, payload) = read_header(bytes)?;
    check(&h)?;
    let (image, reference) = match h.compression {
        COMPRESSION_NONE => {
            if payload.len() != h.image_len as usize {
                return Err("truncated snapshot".into());
            }
            (payload.to_vec(), None)
        }
        COMPRESSION_ZSTD => {
            let reference = resolve_reference(&h, reference)?;
            let image = zstd::bulk::Decompressor::with_dictionary(&reference)
                .and_then(|mut d| d.decompress(payload, h.image_len as usize))
                .map_err(|e| format!("corrupt snapshot: {e}"))?;
            if image.len() != h.image_len as usize {
                return Err("corrupt snapshot: wrong image length".into());
            }
            (image, Some(reference))
        }
        n => return Err(format!("unknown snapshot compression {n}")),
    };
    let image = match h.against {
        AGAINST_BASE => expand(&image, djs_prelude::BASE)?,
        AGAINST_REFERENCE => expand(
            &image,
            reference
                .as_deref()
                .ok_or("corrupt snapshot: no reference")?,
        )?,
        n => return Err(format!("corrupt snapshot: unknown prefix base {n}")),
    };
    Ok(Snapshot {
        format: Format::Container,
        header: Some(h),
        meta: Some(meta),
        image,
        reference,
    })
}

/// A session image completed with its base (this build's base image, or a reference).
fn expand(s: &[u8], base: &[u8]) -> Result<Vec<u8>, String> {
    let raw = session::expand_session(s, base).map_err(|e| match e {
        session::SessionError::BaseMismatch => {
            "the session image was made against another base image".to_string()
        }
        e => format!("corrupt snapshot: {}", e.text()),
    })?;
    Ok(raw)
}

/// A container's header, metadata and payload, checked for length.
pub fn read_header(bytes: &[u8]) -> Result<(Header, String, &[u8]), String> {
    if u32_at(bytes, 0) != Some(CONTAINER_MAGIC) {
        return Err("not a snapshot container".into());
    }
    let format = bytes
        .get(4..6)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or("truncated snapshot")?;
    if format != FORMAT {
        return Err(format!(
            "snapshot container format {format}; this djs reads format {FORMAT}"
        ));
    }
    if bytes.len() < HEADER {
        return Err("truncated snapshot".into());
    }
    let word = |at| u32_at(bytes, at).unwrap();
    let h = Header {
        app: bytes[6],
        compression: bytes[7],
        deps: ImageDeps {
            layout: word(8),
            features: word(12),
            compat: word(16),
            ops: word(20),
            intrinsics: word(24),
            records: word(28),
        },
        prefix: (word(36) as u64) << 32 | word(32) as u64,
        reference_kind: bytes[40],
        reference_hash: (word(48) as u64) << 32 | word(44) as u64,
        reference_len: word(52),
        image_len: word(56),
        payload_len: word(64),
        against: bytes[41],
    };
    let meta_len = word(60) as usize;
    if bytes.len() != HEADER + meta_len + h.payload_len as usize {
        return Err("truncated snapshot".into());
    }
    let meta = std::str::from_utf8(&bytes[HEADER..HEADER + meta_len])
        .map_err(|_| "corrupt snapshot: its metadata is not UTF-8")?;
    Ok((h, meta.to_string(), &bytes[HEADER + meta_len..]))
}

/// This build's base prefix hash.
fn our_prefix() -> u64 {
    session::prefix_hash(djs_prelude::BASE).expect("the base image has a builtin prefix")
}

/// Checks a container's dependency descriptor against this build's (D2265), before
/// anything is decompressed: the first component that differs is the error.
pub fn check(h: &Header) -> Result<(), String> {
    if let Some((dep, found, expected)) = h.deps.first_difference(&IMAGE_DEPS) {
        return Err(incompatible(
            dep.name(),
            &dep.show(found),
            &dep.show(expected),
        ));
    }
    let ours = our_prefix();
    if h.prefix != ours {
        return Err(incompatible(
            "base image",
            &format!("{:016x}", h.prefix),
            &format!("{ours:016x}"),
        ));
    }
    Ok(())
}

/// js/src/snapshot.ts `checkSnapshot`'s message.
fn incompatible(what: &str, found: &str, expected: &str) -> String {
    format!(
        "the snapshot is from a djs with another {what} (snapshot: {found}; this djs: {expected}): \
         it restores only into a djs with the same {what}"
    )
}

/// The reference image of a compressed container: the one given (checked against the
/// header), else this build's base when the header names a matching one.
fn resolve_reference(h: &Header, given: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let name = reference_name(h.reference_kind);
    if let Some(r) = given {
        let r = image_of(r)?;
        if fnv1a64(&r) != h.reference_hash || r.len() != h.reference_len as usize {
            return Err(format!(
                "the snapshot is compressed against {name} {:016x}; the given reference is {:016x} ({} bytes)",
                h.reference_hash,
                fnv1a64(&r),
                r.len()
            ));
        }
        return Ok(r);
    }
    if h.reference_kind == REFERENCE_BASE {
        let base = djs_prelude::BASE;
        if fnv1a64(base) == h.reference_hash && base.len() == h.reference_len as usize {
            return Ok(base.to_vec());
        }
        return Err(format!(
            "the snapshot is compressed against the base image {:016x}, and this djs's base is {:016x} \
             (another build): give the module's base image (node scripts/heap/reference.ts base)",
            h.reference_hash,
            fnv1a64(base)
        ));
    }
    Err(format!(
        "the snapshot is compressed against {name} {:016x}: give it as the reference \
         (pi's prelude image: node scripts/heap/reference.ts pi-prelude)",
        h.reference_hash
    ))
}

/// The image in a reference file: a raw image or an uncompressed container.
pub fn image_of(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if is_image(bytes) {
        return Ok(bytes.to_vec());
    }
    let s = read(bytes, None).map_err(|e| format!("the reference: {e}"))?;
    Ok(s.image)
}

/// A container as js/src/snapshot.ts writes it: the session image of `image` (a VM made
/// from this build's base), compressed with zstd at `level` against `reference` (kind,
/// bytes), or stored as it is without one. The header gets the image's descriptor.
pub fn write(
    app: u8,
    meta: &str,
    image: &[u8],
    reference: Option<(u8, &[u8])>,
    level: i32,
) -> Result<Vec<u8>, String> {
    // The image's own part only (D2066): its prefix words against the reference if that
    // has the same prefix (it holds them in place), else against this build's base.
    let deps = ImageDeps::of(image).ok_or("not a heap image")?;
    let (against, image) = match reference.and_then(|r| session::session_image(image, r.1).ok()) {
        Some(s) => (AGAINST_REFERENCE, s),
        None => (
            AGAINST_BASE,
            session::session_image(image, djs_prelude::BASE).map_err(|e| {
                format!(
                    "the image is not one of a VM made from this build's base: {}",
                    e.text()
                )
            })?,
        ),
    };
    let image = image.as_slice();
    let payload = match reference {
        Some((_, r)) => compress(image, r, level).map_err(|e| format!("zstd: {e}"))?,
        None => image.to_vec(),
    };
    let mut out = Vec::with_capacity(HEADER + meta.len() + payload.len());
    out.extend_from_slice(&CONTAINER_MAGIC.to_le_bytes());
    out.extend_from_slice(&FORMAT.to_le_bytes());
    out.push(app);
    out.push(if reference.is_some() {
        COMPRESSION_ZSTD
    } else {
        COMPRESSION_NONE
    });
    for dep in ImageDep::ALL {
        out.extend_from_slice(&deps.get(dep).to_le_bytes());
    }
    let prefix = our_prefix();
    out.extend_from_slice(&(prefix as u32).to_le_bytes());
    out.extend_from_slice(&((prefix >> 32) as u32).to_le_bytes());
    out.push(reference.map_or(REFERENCE_NONE, |r| r.0));
    out.push(against);
    out.extend_from_slice(&[0, 0]);
    let hash = reference.map_or(0, |r| fnv1a64(r.1));
    out.extend_from_slice(&(hash as u32).to_le_bytes());
    out.extend_from_slice(&((hash >> 32) as u32).to_le_bytes());
    out.extend_from_slice(&(reference.map_or(0, |r| r.1.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(image.len() as u32).to_le_bytes());
    out.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(meta.as_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

/// zstd with `reference` as a raw-content dictionary, a content checksum and a window
/// covering the reference and the image (D2261).
fn compress(image: &[u8], reference: &[u8], level: i32) -> std::io::Result<Vec<u8>> {
    use zstd::zstd_safe::CParameter;
    let mut c = zstd::bulk::Compressor::with_dictionary(level, reference)?;
    c.set_parameter(CParameter::ChecksumFlag(true))?;
    let n = reference.len() + image.len();
    let window = (10..31).find(|&l| (1usize << l) >= n).unwrap_or(31);
    c.set_parameter(CParameter::WindowLog(window))?;
    c.compress(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_matches_the_js_host() {
        // js/src/snapshot.ts fnv1a64("") / ("a") / ("foobar").
        assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x85944171f73967e8);
    }

    fn image() -> Vec<u8> {
        let mut vm = djs_prelude::new_vm(djs_core::vm::VmOptions::default()).unwrap();
        let g = vm.global();
        let o = vm.new_object().unwrap();
        vm.define(g, "probe", djs_core::value::Value::object(o))
            .unwrap();
        vm.collect().unwrap();
        vm.bytes().to_vec()
    }

    #[test]
    fn containers_round_trip() {
        let img = image();
        let base = djs_prelude::BASE;
        let meta = r#"{"functions":[]}"#;
        let z = write(
            APP_SANDBOX,
            meta,
            &img,
            Some((REFERENCE_BASE, base)),
            DEFAULT_LEVEL,
        )
        .unwrap();
        assert!(
            z.len() * 4 < img.len(),
            "{} vs {} bytes",
            z.len(),
            img.len()
        );
        // The header as js/src/snapshot.ts lays it out.
        assert_eq!(&z[0..4], b"DJSN");
        assert_eq!(u16::from_le_bytes([z[4], z[5]]), FORMAT);
        assert_eq!(
            (z[6], z[7], z[40], z[41]),
            (
                APP_SANDBOX,
                COMPRESSION_ZSTD,
                REFERENCE_BASE,
                AGAINST_REFERENCE
            )
        );
        // The stored image is the session image (D2066), made against this build's base.
        let session = session::session_image(&img, base).unwrap();
        assert_eq!(u32_at(&z, 56), Some(session.len() as u32));
        let s = read(&z, None).unwrap();
        assert_eq!(s.format, Format::Container);
        assert_eq!(s.image, img);
        assert_eq!(s.meta.as_deref(), Some(meta));
        let h = s.header.unwrap();
        assert_eq!(h.reference_hash, fnv1a64(base));
        // The image's dependency descriptor and its base's prefix (D2265).
        assert_eq!(h.deps, IMAGE_DEPS);
        assert_eq!(Some(h.prefix), session::prefix_hash(base));
        // Uncompressed: the session image against the base.
        let plain = write(APP_PI_SESSION, "{}", &img, None, DEFAULT_LEVEL).unwrap();
        assert_eq!(plain[41], AGAINST_BASE);
        let s = read(&plain, None).unwrap();
        assert_eq!(s.image, img);
        // A session image or a raw image on its own reads too.
        assert_eq!(read(&session, None).unwrap().format, Format::SessionImage);
        assert_eq!(read(&session, None).unwrap().image, img);
        assert_eq!(read(&img, None).unwrap().format, Format::RawImage);
        // Another container version is refused (D024).
        let mut other = plain.clone();
        other[4] = 1;
        assert!(
            read(&other, None)
                .unwrap_err()
                .contains("snapshot container format 1; this djs reads format 3")
        );
        // Each component of the descriptor is refused by name, and so is another base.
        for (at, what) in [
            (8, "cell layout"),
            (12, "feature set"),
            (16, "compatibility version"),
            (20, "opcode table"),
            (24, "intrinsic table"),
            (28, "internal id tables"),
            (32, "base image"),
        ] {
            let mut other = plain.clone();
            other[at] ^= 1;
            let e = read(&other, None).unwrap_err();
            assert!(
                e.starts_with(&format!(
                    "the snapshot is from a djs with another {what} (snapshot: "
                )),
                "{e}"
            );
        }
        // An image without the base's prefix makes no container.
        let mut bare = djs_core::vm::Vm::new(djs_core::vm::VmOptions::default()).unwrap();
        bare.collect().unwrap();
        assert!(write(APP_REPL, "{}", bare.bytes(), None, DEFAULT_LEVEL).is_err());
    }

    #[test]
    fn bad_containers_are_refused() {
        let img = image();
        let z = write(
            APP_SANDBOX,
            "{}",
            &img,
            Some((REFERENCE_PI_PRELUDE, djs_prelude::BASE)),
            DEFAULT_LEVEL,
        )
        .unwrap();
        // A prelude reference must be given; a wrong one is refused by its hash.
        assert!(read(&z, None).unwrap_err().contains("pi's prelude image"));
        assert!(
            read(&z, Some(&img))
                .unwrap_err()
                .contains("the given reference is")
        );
        assert_eq!(read(&z, Some(djs_prelude::BASE)).unwrap().image, img);
        assert!(
            read(&z[..z.len() - 1], None)
                .unwrap_err()
                .contains("truncated")
        );
        let mut damaged = z.clone();
        let n = damaged.len();
        damaged[n - 10] ^= 0xff;
        assert!(
            read(&damaged, Some(djs_prelude::BASE))
                .unwrap_err()
                .contains("corrupt snapshot")
        );
        assert!(read(b"nope nope", None).is_err());
    }
}

#[cfg(test)]
mod cross {
    /// Writes a container for js/src/snapshot.ts to read (a manual cross-check:
    /// `DJS_CONTAINER_OUT=f cargo test -p djs-cli write_for_node -- --ignored`).
    #[test]
    #[ignore]
    fn write_for_node() {
        let Ok(out) = std::env::var("DJS_CONTAINER_OUT") else {
            return;
        };
        let mut vm = djs_prelude::new_vm(djs_core::vm::VmOptions::default()).unwrap();
        vm.collect().unwrap();
        let c = super::write(
            super::APP_SANDBOX,
            r#"{"functions":[]}"#,
            vm.bytes(),
            Some((super::REFERENCE_BASE, djs_prelude::BASE)),
            3,
        )
        .unwrap();
        std::fs::write(out, c).unwrap();
    }
}
