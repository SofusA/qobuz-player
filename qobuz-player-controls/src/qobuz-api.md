# Qobuz Streaming API (qbz-1)

Reverse-engineered from the Qobuz web player (bundle.js). This documents the newer
segmented streaming API that replaces the old `track/getFileUrl` endpoint.

## Endpoints

### `POST /session/start`

Starts a new streaming session. Must be called before `file/url`.

**Content-Type**: `application/x-www-form-urlencoded`

| Parameter | Value |
|-----------|-------|
| `profile` | `"qbz-1"` (fixed) |

Request is signed (same signature scheme as other API calls).

**Response**:
```json
{
  "session_id": "abc123...",
  "infos": "hUnLXvp0zbBQ3bE3XplTLw.bm9uZQ"
}
```

- `session_id` — used as `X-Session-Id` header in subsequent requests
- `infos` — base64-encoded values used for key derivation (see Encryption below)

### `GET /file/url`

Returns a segmented streaming URL for a track.

| Parameter | Description |
|-----------|-------------|
| `track_id` | Track ID (integer) |
| `format_id` | Audio format (see Format IDs below) |
| `intent` | Streaming intent (e.g. `"stream"`) |

**Headers**: `X-Session-Id: {session_id}`

Request is signed with query string included in signature.

**Response**:
```json
{
  "url_template": "https://streaming.qobuz.com/.../segment/$SEGMENT$",
  "mime_type": "audio/mp4; codecs=\"flac\"",
  "n_segments": 26,
  "key_id": "...",
  "key": "...",
  "sampling_rate": 96000,
  "duration": 312.5,
  "n_samples": 30000000
}
```

- `url_template` — URL with `$SEGMENT$` placeholder (replace with 0, 1, 2, ..., n_segments-1)
- `n_segments` — total segments including init segment (segment 0)
- `key_id` — identifies the encryption key
- `key` — encrypted content key (base64-encoded, see Encryption below)

### Format IDs

| ID | Quality |
|----|---------|
| 5 | MP3 320kbps |
| 6 | CD (16-bit/44.1kHz FLAC) |
| 7 | Hi-Res 96kHz (24-bit/96kHz FLAC) |
| 27 | Hi-Res 192kHz (24-bit/192kHz FLAC) |

## Segment Structure (CMAF/fMP4)

All segments are CMAF (Common Media Application Format) fragments.

### Segment 0 — Init Segment

Contains metadata needed to decode all subsequent segments:
- `ftyp` box — file type
- Custom UUID box (`QBZ_INIT_UUID`: `c7c75df0fdd951e98fc22971e4acf8d2`):
  - FLAC STREAMINFO (sample rate, channels, bit depth)
  - Complete FLAC header (fLaC magic + STREAMINFO block)
  - `key_id` for encryption
  - **Segment table**: per-segment `{byte_len: u32, sample_count: u32}` — exact decrypted FLAC frame sizes, enabling byte-offset to segment-index mapping for seeking
- `moov` box — standard MP4 metadata

### Segments 1..N — Audio Segments

Each audio segment contains:
- `styp` box — segment type
- Custom UUID box (`QBZ_SEGMENT_UUID`: `3b42129256f35f75923663b69a1f52b2`):
  - Per-frame entries: `[4B size][2B skip][2B flags][8B iv]`
  - `flags = 0` → frame is unencrypted (cleartext FLAC)
  - `flags != 0` → frame is encrypted (AES-128-CTR)
  - `iv` — 8 bytes used as nonce for CTR decryption
- `moof` box — movie fragment header
- `mdat` box — actual audio data (FLAC frames, possibly encrypted)

### Segment Sizes

Segments are fixed in count and size per track — there is no API parameter to request
smaller segments. Typical sizes:

- CD quality: ~2-4 MB per segment
- Hi-Res 96kHz: ~5-8 MB per segment
- Hi-Res 192kHz: ~8-15 MB per segment

No HTTP range request support — each segment must be downloaded in full.

## Encryption Scheme (qbz-1)

Three-step key derivation, then per-frame AES-128-CTR decryption.

### Step 1: Session Key (HKDF-SHA256)

```
rng_init = hex_decode("abb21364945c0583309667d13ca3d93a")  // 16 bytes, from bundle.js
infos_parts = session_infos.split(".")  // e.g. "hUnLXvp0...Lw.bm9uZQ"
salt = base64_decode(infos_parts[0])
info = base64_decode(infos_parts[1])
session_key = HKDF-SHA256(ikm=rng_init, salt=salt, info=info, len=16)
```

The `rng_init` value is derived from `initialSeed("YWJiMjEzNjQ5NDVjMDU4MzMwOTY2N2", window.utimezone.berlin)` in the JavaScript bundle.

### Step 2: Content Key Unwrap (AES-128-CBC)

```
key_parts = track_url.key.split(".")  // 3 parts, all base64
iv = base64_decode(key_parts[2])
encrypted_key = base64_decode(key_parts[1])
content_key = AES-128-CBC-decrypt(session_key, iv, encrypted_key)  // PKCS7 unpad → 16 bytes
```

### Step 3: Per-Frame Decryption (AES-128-CTR)

For each encrypted frame in a segment (identified by `flags != 0` in the QBZ_SEGMENT_UUID box):

```
nonce = [8_byte_iv_from_frame_entry || 0x00 * 8]  // 16 bytes total
plaintext_flac_frame = AES-128-CTR(content_key, nonce, encrypted_frame_bytes)
```

Unencrypted frames (`flags = 0`) are passed through as-is.

### Reassembly

The decrypted output is a standard FLAC stream:
1. FLAC header (from init segment's QBZ_INIT_UUID box)
2. Concatenated decrypted frames from segments 1..N

This can be written directly to a `.flac` file for caching.

## Web Player Implementation

The Qobuz web player uses Media Source Extensions (MSE) with `SourceBuffer` in
`"sequence"` mode. Segments are appended to a `BufferQueue` as they download.

Key constants from bundle.js:
```javascript
SEGMENT_TEMPLATE_PLACEHOLDER = "$SEGMENT$"
QBZ_INIT_UUID = "c7c75df0fdd951e98fc22971e4acf8d2"
QBZ_SEGMENT_UUID = "3b42129256f35f75923663b69a1f52b2"

HLS_FILE_TYPE = { PREVIEW: "preview", FULL: "full" }
```

The web player tracks download state per segment with a retry strategy array.
