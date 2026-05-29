# rawshift-video

Video format support for [rawshift](https://github.com/justin13888/rawshift).

> **Status: planned, not yet implemented.** No video code ships today. This
> crate exists so the feature tree, public API, and the
> [`rawshift`](https://crates.io/crates/rawshift) facade can be laid out ahead
> of the decoder work.

## Roadmap

The formats below are prioritised by the cameras in rawshift's supported device
list:

| Format / Codec       | Container       | Status  | Notes                                          |
| -------------------- | --------------- | ------- | ---------------------------------------------- |
| XAVC HS (H.265/HEVC) | MP4             | Planned | Sony mirrorless video.                         |
| XAVC S (H.264/AVC)   | MP4             | Planned | Sony mirrorless video.                         |
| Apple ProRes         | QuickTime (MOV) | Planned | iPhone Pro and professional editing workflows. |
| HEVC (H.265)         | QuickTime (MOV) | Planned | Default iPhone video.                          |
| H.264 (AVC)          | QuickTime (MOV) | Planned | Legacy and compatibility video.                |

Initial work will focus on container parsing and metadata extraction, reusing
the in-repo ISOBMFF parser already used for Canon CR3 (both MP4 and QuickTime
are ISOBMFF-based). Codec-level decoding is a later milestone.

## Feature Flags

Video features mirror the `rawshift-image` tier structure but currently gate no
code or dependencies — they exist so the surface is laid out ahead of the
decoder work.

- **Bundles** — `video` (all formats), `full`.
- **Formats** — `xavc-hs`, `xavc-s`, `hevc`, `h264`, `prores`.
- **Directions** — `xavc-hs-decode`, `xavc-s-decode`, `hevc-decode`,
  `h264-decode`, `prores-decode` (decode-only for now).

## License

Licensed under [MPL-2.0](../../LICENSE).
