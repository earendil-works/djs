//! What `djs heap` reads: a raw heap image (`Vm::bytes`), the snapshot formats before the
//! container (`DJSS` of the Sandbox, `DJPI` of the pi adapter), and the snapshot container
//! (`DJSN`, D2260: layout in docs/host.md "Snapshots"), decompressed with its reference
//! image.

use djs_core::heap::cells::{MAGIC, Roots};

/// "DJSN", "DJSS", "DJPI", little-endian.
const CONTAINER_MAGIC: u32 = 0x4e53_4a44;
const SANDBOX_V1_MAGIC: u32 = 0x5353_4a44;
const PI_V1_MAGIC: u32 = 0x4950_4a44;
const CONTAINER_HEADER: usize = 40;

pub const REFERENCE_NONE: u8 = 0;
pub const REFERENCE_BASE: u8 = 1;
pub const REFERENCE_PI_PRELUDE: u8 = 2;

/// The header fields of a snapshot container.
#[derive(Debug, Clone)]
pub struct Container {
    pub app: u8,
    pub compression: u8,
    pub build: u32,
    pub reference_kind: u8,
    pub reference_hash: u64,
    pub reference_len: u32,
    pub image_len: u32,
    pub payload_len: u32,
}

impl Container {
    pub fn app_name(&self) -> String {
        match self.app {
            1 => "sandbox".into(),
            2 => "pi session".into(),
            n => format!("app {n}"),
        }
    }
}

pub fn reference_name(kind: u8) -> &'static str {
    match kind {
        REFERENCE_NONE => "none",
        REFERENCE_BASE => "the base image",
        REFERENCE_PI_PRELUDE => "pi's prelude image",
        _ => "an unknown reference",
    }
}

/// A heap dump, unwrapped.
pub struct Dump {
    /// How it was stored: "raw image", "DJSS (sandbox, v1)", "DJSN container (...)".
    pub format: String,
    /// The file's size.
    pub file_len: usize,
    pub container: Option<Container>,
    /// The host's metadata (JSON text) of a snapshot.
    pub meta: Option<String>,
    pub image: Vec<u8>,
    /// The reference image the container was compressed against, if any.
    pub reference: Option<Vec<u8>>,
}

/// FNV-1a 64 (the snapshot container's reference hash, D2260).
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
    bytes.first() == Some(&(djs_core::heap::CellKind::Roots as u8))
        && u32_at(bytes, Roots::MAGIC.offset as usize) == Some(MAGIC)
}

/// Unwraps `bytes`. `reference` is the reference image the user gave (`--reference`);
/// without one, a container against the base image uses this build's base if the hash
/// matches.
pub fn read(bytes: Vec<u8>, reference: Option<Vec<u8>>) -> Result<Dump, String> {
    let file_len = bytes.len();
    if is_image(&bytes) {
        return Ok(Dump {
            format: "raw image".into(),
            file_len,
            container: None,
            meta: None,
            image: bytes,
            reference: None,
        });
    }
    let magic = u32_at(&bytes, 0).ok_or("not a heap image or snapshot: too short")?;
    if magic == SANDBOX_V1_MAGIC || magic == PI_V1_MAGIC {
        let version = u32_at(&bytes, 4).ok_or("truncated snapshot")?;
        if version != 1 {
            return Err(format!("unknown snapshot version {version}"));
        }
        let meta_len = u32_at(&bytes, 8).ok_or("truncated snapshot")? as usize;
        let meta = bytes.get(12..12 + meta_len).ok_or("truncated snapshot")?;
        let meta = String::from_utf8_lossy(meta).into_owned();
        let image = bytes[12 + meta_len..].to_vec();
        let app = if magic == SANDBOX_V1_MAGIC {
            "sandbox"
        } else {
            "pi session"
        };
        return Ok(Dump {
            format: format!(
                "{} snapshot before the container ({app})",
                if magic == SANDBOX_V1_MAGIC {
                    "DJSS"
                } else {
                    "DJPI"
                }
            ),
            file_len,
            container: None,
            meta: Some(meta),
            image,
            reference: None,
        });
    }
    if magic != CONTAINER_MAGIC {
        return Err(
            "not a heap image or snapshot (no image roots, no DJSN/DJSS/DJPI magic)".into(),
        );
    }
    if bytes.len() < CONTAINER_HEADER {
        return Err("truncated snapshot container".into());
    }
    let format = u16::from_le_bytes([bytes[4], bytes[5]]);
    if format != 1 {
        return Err(format!(
            "snapshot container format {format}; this tool reads format 1"
        ));
    }
    let lo = u32_at(&bytes, 16).unwrap() as u64;
    let hi = u32_at(&bytes, 20).unwrap() as u64;
    let c = Container {
        app: bytes[6],
        compression: bytes[7],
        build: u32_at(&bytes, 8).unwrap(),
        reference_kind: bytes[12],
        reference_hash: hi << 32 | lo,
        reference_len: u32_at(&bytes, 24).unwrap(),
        image_len: u32_at(&bytes, 28).unwrap(),
        payload_len: u32_at(&bytes, 36).unwrap(),
    };
    let meta_len = u32_at(&bytes, 32).unwrap() as usize;
    if bytes.len() != CONTAINER_HEADER + meta_len + c.payload_len as usize {
        return Err("truncated snapshot container".into());
    }
    let meta =
        String::from_utf8_lossy(&bytes[CONTAINER_HEADER..CONTAINER_HEADER + meta_len]).into_owned();
    let payload = &bytes[CONTAINER_HEADER + meta_len..];
    let describe = |what: &str| {
        format!(
            "DJSN container ({}, {what}, {} KB)",
            c.app_name(),
            kb(bytes.len())
        )
    };
    let (format, image, reference) = match c.compression {
        0 => {
            if payload.len() != c.image_len as usize {
                return Err("truncated snapshot container".into());
            }
            (describe("uncompressed"), payload.to_vec(), None)
        }
        1 => {
            let reference = resolve_reference(&c, reference)?;
            let image = zstd::bulk::Decompressor::with_dictionary(&reference)
                .and_then(|mut d| d.decompress(payload, c.image_len as usize))
                .map_err(|e| format!("corrupt snapshot: {e}"))?;
            if image.len() != c.image_len as usize {
                return Err("corrupt snapshot: wrong image length".into());
            }
            (
                describe(&format!(
                    "zstd against {}",
                    reference_name(c.reference_kind)
                )),
                image,
                Some(reference),
            )
        }
        n => return Err(format!("unknown snapshot compression {n}")),
    };
    Ok(Dump {
        format,
        file_len,
        container: Some(c),
        meta: Some(meta),
        image,
        reference,
    })
}

/// The reference image of a compressed container: the one given (checked against the
/// header's hash), else this build's base image when the header names a matching one.
fn resolve_reference(c: &Container, given: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    let name = reference_name(c.reference_kind);
    if let Some(r) = given {
        let r = unwrap_reference(r)?;
        if fnv1a64(&r) != c.reference_hash || r.len() != c.reference_len as usize {
            return Err(format!(
                "the snapshot is compressed against {name} {:016x}; --reference is {:016x} ({} bytes)",
                c.reference_hash,
                fnv1a64(&r),
                r.len()
            ));
        }
        return Ok(r);
    }
    if c.reference_kind == REFERENCE_BASE {
        let base = djs_prelude::BASE;
        if fnv1a64(base) == c.reference_hash && base.len() == c.reference_len as usize {
            return Ok(base.to_vec());
        }
        return Err(format!(
            "the snapshot is compressed against the base image {:016x}, and this djs's base is {:016x} \
             (another build): pass the module's base with --reference \
             (node scripts/heap/reference.ts base --wasm MODULE -o base.img)",
            c.reference_hash,
            fnv1a64(base)
        ));
    }
    Err(format!(
        "the snapshot is compressed against {name} {:016x}: pass it with --reference \
         (pi's prelude image: node scripts/heap/reference.ts pi-prelude --data START.json -o prelude.img)",
        c.reference_hash
    ))
}

/// A reference given as a file: a raw image, or an uncompressed container of one.
pub fn unwrap_reference(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    if is_image(&bytes) {
        return Ok(bytes);
    }
    let d = read(bytes, None).map_err(|e| format!("--reference: {e}"))?;
    Ok(d.image)
}

pub fn kb(n: usize) -> String {
    format!("{:.1}", n as f64 / 1024.0)
}
