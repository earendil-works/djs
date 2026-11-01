//! Snapshot files in Rust: the snapshot container (`DJSN`, D2260; layout in docs/host.md
//! "Snapshots", written by js/src/snapshot.ts), the formats before it (`DJSS` of the
//! Sandbox, `DJPI` of the pi adapter) and raw heap images (`Vm::bytes`), read and written
//! the way the JS host does (D2264).
//!
//! A container's image is a zstd frame compressed with a *reference image* as a
//! raw-content dictionary: this build's base snapshot, or pi's prelude image, which the
//! reader has to be given.

use djs_core::heap::CellKind;
use djs_core::heap::cells::{MAGIC, Roots};

/// "DJSN", "DJSS", "DJPI", little-endian.
pub const CONTAINER_MAGIC: u32 = 0x4e53_4a44;
const SANDBOX_V1_MAGIC: u32 = 0x5353_4a44;
const PI_V1_MAGIC: u32 = 0x4950_4a44;
pub const FORMAT: u16 = 1;
const HEADER: usize = 40;

/// The app byte: what the metadata is.
pub const APP_SANDBOX: u8 = 1;
pub const APP_PI_SESSION: u8 = 2;

pub const REFERENCE_NONE: u8 = 0;
/// This build's base snapshot (`djs_prelude::BASE`).
pub const REFERENCE_BASE: u8 = 1;
/// pi's prelude image: the session's VM right after pi's prelude, sealed.
pub const REFERENCE_PI_PRELUDE: u8 = 2;

pub const COMPRESSION_NONE: u8 = 0;
pub const COMPRESSION_ZSTD: u8 = 1;

/// js/src/snapshot.ts's `DEFAULT_ZSTD_LEVEL` (D2261).
#[allow(dead_code)] // the writer's: the REPL's `.snapshot` next
pub const DEFAULT_LEVEL: i32 = 3;

/// The header fields of a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub app: u8,
    pub compression: u8,
    /// The engine build hash (`djs_build_hash`, `vm::BUILD_HASH`).
    pub build: u32,
    pub reference_kind: u8,
    /// FNV-1a 64 of the reference image.
    pub reference_hash: u64,
    pub reference_len: u32,
    pub image_len: u32,
    pub payload_len: u32,
}

/// How a snapshot file was stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    RawImage,
    /// The Sandbox's format before the container.
    SandboxV1,
    /// The pi adapter's format before the container.
    PiV1,
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
            n => format!("app {n}"),
        }
    }
}

impl Snapshot {
    /// "raw image", "DJSN container (pi session, zstd against pi's prelude image)", ...
    pub fn describe(&self) -> String {
        match (self.format, &self.header) {
            (Format::RawImage, _) => "raw image".into(),
            (Format::SandboxV1, _) => "DJSS snapshot before the container (sandbox)".into(),
            (Format::PiV1, _) => "DJPI snapshot before the container (pi session)".into(),
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
    let magic = u32_at(bytes, 0).ok_or("not a heap image or snapshot: too short")?;
    if magic == SANDBOX_V1_MAGIC || magic == PI_V1_MAGIC {
        let version = u32_at(bytes, 4).ok_or("truncated snapshot")?;
        if version != 1 {
            return Err(format!("unknown snapshot version {version}"));
        }
        let meta_len = u32_at(bytes, 8).ok_or("truncated snapshot")? as usize;
        let meta = bytes.get(12..12 + meta_len).ok_or("truncated snapshot")?;
        return Ok(Snapshot {
            format: if magic == SANDBOX_V1_MAGIC {
                Format::SandboxV1
            } else {
                Format::PiV1
            },
            header: None,
            meta: Some(String::from_utf8_lossy(meta).into_owned()),
            image: bytes[12 + meta_len..].to_vec(),
            reference: None,
        });
    }
    if magic != CONTAINER_MAGIC {
        return Err(
            "not a heap image or snapshot (no image roots, no DJSN/DJSS/DJPI magic)".into(),
        );
    }
    let (h, meta, payload) = read_header(bytes)?;
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
    Ok(Snapshot {
        format: Format::Container,
        header: Some(h),
        meta: Some(meta),
        image,
        reference,
    })
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
        build: word(8),
        reference_kind: bytes[12],
        reference_hash: (word(20) as u64) << 32 | word(16) as u64,
        reference_len: word(24),
        image_len: word(28),
        payload_len: word(36),
    };
    let meta_len = word(32) as usize;
    if bytes.len() != HEADER + meta_len + h.payload_len as usize {
        return Err("truncated snapshot".into());
    }
    let meta = std::str::from_utf8(&bytes[HEADER..HEADER + meta_len])
        .map_err(|_| "corrupt snapshot: its metadata is not UTF-8")?;
    Ok((h, meta.to_string(), &bytes[HEADER + meta_len..]))
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

/// A container as js/src/snapshot.ts writes it: `image` compressed with zstd at `level`
/// against `reference` (kind, bytes), or stored as it is without one.
#[allow(dead_code)] // the REPL's `.snapshot` next; the tests now
pub fn write(
    app: u8,
    build: u32,
    meta: &str,
    image: &[u8],
    reference: Option<(u8, &[u8])>,
    level: i32,
) -> Result<Vec<u8>, String> {
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
    out.extend_from_slice(&build.to_le_bytes());
    out.push(reference.map_or(REFERENCE_NONE, |r| r.0));
    out.extend_from_slice(&[0, 0, 0]);
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
#[allow(dead_code)]
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
            djs_core::vm::BUILD_HASH,
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
        assert_eq!(u16::from_le_bytes([z[4], z[5]]), 1);
        assert_eq!(
            (z[6], z[7], z[12]),
            (APP_SANDBOX, COMPRESSION_ZSTD, REFERENCE_BASE)
        );
        assert_eq!(u32_at(&z, 28), Some(img.len() as u32));
        let s = read(&z, None).unwrap();
        assert_eq!(s.format, Format::Container);
        assert_eq!(s.image, img);
        assert_eq!(s.meta.as_deref(), Some(meta));
        assert_eq!(s.header.unwrap().reference_hash, fnv1a64(base));
        // Uncompressed, and the older formats.
        let plain = write(APP_PI_SESSION, 7, "{}", &img, None, DEFAULT_LEVEL).unwrap();
        let s = read(&plain, None).unwrap();
        assert_eq!((s.image == img, s.header.unwrap().build), (true, 7));
        let mut old = b"DJSS".to_vec();
        old.extend_from_slice(&1u32.to_le_bytes());
        old.extend_from_slice(&2u32.to_le_bytes());
        old.extend_from_slice(b"{}");
        old.extend_from_slice(&img);
        let s = read(&old, None).unwrap();
        assert_eq!((s.format, s.image == img), (Format::SandboxV1, true));
        assert_eq!(read(&img, None).unwrap().format, Format::RawImage);
    }

    #[test]
    fn bad_containers_are_refused() {
        let img = image();
        let z = write(
            APP_SANDBOX,
            1,
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
            djs_core::vm::BUILD_HASH,
            r#"{"functions":[]}"#,
            vm.bytes(),
            Some((super::REFERENCE_BASE, djs_prelude::BASE)),
            3,
        )
        .unwrap();
        std::fs::write(out, c).unwrap();
    }
}
