# HLS Stream Support

CinaVault Premium recognizes direct HLS playlist URLs ending in `.m3u8` (including query-string variants).

The Downloads workflow routes HLS downloads through yt-dlp with FFmpeg enabled as the segment downloader and remux/merge engine. The Downloads UI detects HLS URLs and exposes a dedicated **Decode & Download HLS** action.

Supported behavior is limited to sources the user is authorized to access and to streams that are not protected by DRM. Browser challenges and CAPTCHAs remain manual user interactions.
