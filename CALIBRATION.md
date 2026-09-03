# Calibration

- Machine: Mac Studio (Mac15,14)
- CPU: Apple M3 Ultra, 28 cores
- Operating system: macOS 26.6.1 (25G76)
- Rust compiler: rustc 1.98.0 (88d9e12ae 2026-08-18) (Homebrew)
- libwebp: 1.6.0

Command: `PATH=/opt/homebrew/bin:$PATH cargo run --example bench`

```text
tiny-webp 0.0.0 on macos aarch64
flat q50 megapixels_per_second=1.711 peak_heap_bytes_per_pixel=4.265 bytes=56 rgb_psnr_db=48.131 cwebp_bytes=70 tiny_webp_to_cwebp_size_ratio=0.800 cwebp_rgb_psnr_db=47.291 cwebp_subprocess_ms=2.474
checker q50 megapixels_per_second=1.127 peak_heap_bytes_per_pixel=5.526 bytes=702 rgb_psnr_db=42.433 cwebp_bytes=320 tiny_webp_to_cwebp_size_ratio=2.194 cwebp_rgb_psnr_db=41.726 cwebp_subprocess_ms=2.427
gradient q50 megapixels_per_second=1.522 peak_heap_bytes_per_pixel=4.288 bytes=352 rgb_psnr_db=35.329 cwebp_bytes=192 tiny_webp_to_cwebp_size_ratio=1.833 cwebp_rgb_psnr_db=38.151 cwebp_subprocess_ms=2.484
text-blocks q50 megapixels_per_second=1.188 peak_heap_bytes_per_pixel=5.297 bytes=1902 rgb_psnr_db=35.797 cwebp_bytes=902 tiny_webp_to_cwebp_size_ratio=2.109 cwebp_rgb_psnr_db=34.881 cwebp_subprocess_ms=2.627
noise q50 megapixels_per_second=0.851 peak_heap_bytes_per_pixel=6.041 bytes=3044 rgb_psnr_db=12.684 cwebp_bytes=1850 tiny_webp_to_cwebp_size_ratio=1.645 cwebp_rgb_psnr_db=12.688 cwebp_subprocess_ms=2.910
lowpass-noise q50 megapixels_per_second=1.379 peak_heap_bytes_per_pixel=4.447 bytes=596 rgb_psnr_db=28.934 cwebp_bytes=454 tiny_webp_to_cwebp_size_ratio=1.313 cwebp_rgb_psnr_db=28.196 cwebp_subprocess_ms=2.676
alpha-soft q50 megapixels_per_second=1.539 peak_heap_bytes_per_pixel=5.297 bytes=3452 rgb_psnr_db=35.329 cwebp_bytes=662 tiny_webp_to_cwebp_size_ratio=5.215 cwebp_rgb_psnr_db=38.018 cwebp_subprocess_ms=3.768
alpha-hard q50 megapixels_per_second=1.552 peak_heap_bytes_per_pixel=5.297 bytes=3452 rgb_psnr_db=35.329 cwebp_bytes=236 tiny_webp_to_cwebp_size_ratio=14.627 cwebp_rgb_psnr_db=12.682 cwebp_subprocess_ms=2.709
alpha-odd q50 megapixels_per_second=0.801 peak_heap_bytes_per_pixel=8.827 bytes=724 rgb_psnr_db=38.550 cwebp_bytes=256 tiny_webp_to_cwebp_size_ratio=2.828 cwebp_rgb_psnr_db=37.166 cwebp_subprocess_ms=2.774
photo-large q50 megapixels_per_second=1.425 peak_heap_bytes_per_pixel=4.341 bytes=132954 rgb_psnr_db=28.907 cwebp_bytes=88650 tiny_webp_to_cwebp_size_ratio=1.500 cwebp_rgb_psnr_db=28.109 cwebp_subprocess_ms=64.672
one-pixel q50 megapixels_per_second=0.006 peak_heap_bytes_per_pixel=1009.000 bytes=42 rgb_psnr_db=inf cwebp_bytes=44 tiny_webp_to_cwebp_size_ratio=0.955 cwebp_rgb_psnr_db=inf cwebp_subprocess_ms=2.569
single-column q50 megapixels_per_second=0.072 peak_heap_bytes_per_pixel=85.152 bytes=112 rgb_psnr_db=35.939 cwebp_bytes=98 tiny_webp_to_cwebp_size_ratio=1.143 cwebp_rgb_psnr_db=37.526 cwebp_subprocess_ms=2.356
single-row q50 megapixels_per_second=0.075 peak_heap_bytes_per_pixel=83.000 bytes=100 rgb_psnr_db=38.351 cwebp_bytes=96 tiny_webp_to_cwebp_size_ratio=1.042 cwebp_rgb_psnr_db=39.756 cwebp_subprocess_ms=2.690
odd-size q50 megapixels_per_second=0.745 peak_heap_bytes_per_pixel=9.127 bytes=526 rgb_psnr_db=35.583 cwebp_bytes=224 tiny_webp_to_cwebp_size_ratio=2.348 cwebp_rgb_psnr_db=33.143 cwebp_subprocess_ms=2.402
flat q75 megapixels_per_second=1.718 peak_heap_bytes_per_pixel=4.269 bytes=58 rgb_psnr_db=45.121 cwebp_bytes=74 tiny_webp_to_cwebp_size_ratio=0.784 cwebp_rgb_psnr_db=47.599 cwebp_subprocess_ms=2.409
checker q75 megapixels_per_second=1.133 peak_heap_bytes_per_pixel=5.574 bytes=726 rgb_psnr_db=41.312 cwebp_bytes=304 tiny_webp_to_cwebp_size_ratio=2.388 cwebp_rgb_psnr_db=46.325 cwebp_subprocess_ms=2.645
gradient q75 megapixels_per_second=1.521 peak_heap_bytes_per_pixel=4.318 bytes=398 rgb_psnr_db=38.508 cwebp_bytes=220 tiny_webp_to_cwebp_size_ratio=1.809 cwebp_rgb_psnr_db=41.104 cwebp_subprocess_ms=2.588
text-blocks q75 megapixels_per_second=1.154 peak_heap_bytes_per_pixel=5.486 bytes=2192 rgb_psnr_db=39.309 cwebp_bytes=1022 tiny_webp_to_cwebp_size_ratio=2.145 cwebp_rgb_psnr_db=37.751 cwebp_subprocess_ms=2.666
noise q75 megapixels_per_second=0.795 peak_heap_bytes_per_pixel=7.581 bytes=3874 rgb_psnr_db=12.753 cwebp_bytes=2170 tiny_webp_to_cwebp_size_ratio=1.785 cwebp_rgb_psnr_db=12.748 cwebp_subprocess_ms=2.882
lowpass-noise q75 megapixels_per_second=1.275 peak_heap_bytes_per_pixel=4.608 bytes=844 rgb_psnr_db=30.470 cwebp_bytes=610 tiny_webp_to_cwebp_size_ratio=1.384 cwebp_rgb_psnr_db=29.753 cwebp_subprocess_ms=2.579
alpha-soft q75 megapixels_per_second=1.519 peak_heap_bytes_per_pixel=5.327 bytes=3498 rgb_psnr_db=38.508 cwebp_bytes=714 tiny_webp_to_cwebp_size_ratio=4.899 cwebp_rgb_psnr_db=40.037 cwebp_subprocess_ms=3.804
alpha-hard q75 megapixels_per_second=1.503 peak_heap_bytes_per_pixel=5.327 bytes=3498 rgb_psnr_db=38.508 cwebp_bytes=262 tiny_webp_to_cwebp_size_ratio=13.351 cwebp_rgb_psnr_db=12.583 cwebp_subprocess_ms=2.650
alpha-odd q75 megapixels_per_second=0.792 peak_heap_bytes_per_pixel=8.918 bytes=748 rgb_psnr_db=38.405 cwebp_bytes=266 tiny_webp_to_cwebp_size_ratio=2.812 cwebp_rgb_psnr_db=35.699 cwebp_subprocess_ms=3.072
photo-large q75 megapixels_per_second=1.337 peak_heap_bytes_per_pixel=4.504 bytes=196866 rgb_psnr_db=30.524 cwebp_bytes=123630 tiny_webp_to_cwebp_size_ratio=1.592 cwebp_rgb_psnr_db=29.515 cwebp_subprocess_ms=68.736
one-pixel q75 megapixels_per_second=0.006 peak_heap_bytes_per_pixel=1009.000 bytes=42 rgb_psnr_db=inf cwebp_bytes=44 tiny_webp_to_cwebp_size_ratio=0.955 cwebp_rgb_psnr_db=inf cwebp_subprocess_ms=2.471
single-column q75 megapixels_per_second=0.072 peak_heap_bytes_per_pixel=86.152 bytes=128 rgb_psnr_db=39.018 cwebp_bytes=110 tiny_webp_to_cwebp_size_ratio=1.164 cwebp_rgb_psnr_db=40.371 cwebp_subprocess_ms=2.359
single-row q75 megapixels_per_second=0.075 peak_heap_bytes_per_pixel=85.576 bytes=110 rgb_psnr_db=40.483 cwebp_bytes=104 tiny_webp_to_cwebp_size_ratio=1.058 cwebp_rgb_psnr_db=42.646 cwebp_subprocess_ms=2.361
odd-size q75 megapixels_per_second=0.673 peak_heap_bytes_per_pixel=10.349 bytes=584 rgb_psnr_db=38.787 cwebp_bytes=252 tiny_webp_to_cwebp_size_ratio=2.317 cwebp_rgb_psnr_db=35.853 cwebp_subprocess_ms=2.364
flat q90 megapixels_per_second=1.737 peak_heap_bytes_per_pixel=4.288 bytes=68 rgb_psnr_db=46.618 cwebp_bytes=76 tiny_webp_to_cwebp_size_ratio=0.895 cwebp_rgb_psnr_db=48.936 cwebp_subprocess_ms=2.732
checker q90 megapixels_per_second=1.173 peak_heap_bytes_per_pixel=5.730 bytes=806 rgb_psnr_db=inf cwebp_bytes=354 tiny_webp_to_cwebp_size_ratio=2.277 cwebp_rgb_psnr_db=51.721 cwebp_subprocess_ms=2.509
gradient q90 megapixels_per_second=1.426 peak_heap_bytes_per_pixel=4.542 bytes=742 rgb_psnr_db=44.627 cwebp_bytes=284 tiny_webp_to_cwebp_size_ratio=2.613 cwebp_rgb_psnr_db=43.377 cwebp_subprocess_ms=2.620
text-blocks q90 megapixels_per_second=1.211 peak_heap_bytes_per_pixel=5.906 bytes=2836 rgb_psnr_db=45.478 cwebp_bytes=1214 tiny_webp_to_cwebp_size_ratio=2.336 cwebp_rgb_psnr_db=44.473 cwebp_subprocess_ms=2.802
noise q90 megapixels_per_second=0.680 peak_heap_bytes_per_pixel=11.159 bytes=6298 rgb_psnr_db=12.792 cwebp_bytes=2950 tiny_webp_to_cwebp_size_ratio=2.135 cwebp_rgb_psnr_db=12.796 cwebp_subprocess_ms=2.804
lowpass-noise q90 megapixels_per_second=1.061 peak_heap_bytes_per_pixel=5.231 bytes=1800 rgb_psnr_db=32.642 cwebp_bytes=1142 tiny_webp_to_cwebp_size_ratio=1.576 cwebp_rgb_psnr_db=32.158 cwebp_subprocess_ms=2.778
alpha-soft q90 megapixels_per_second=1.422 peak_heap_bytes_per_pixel=5.551 bytes=3842 rgb_psnr_db=44.627 cwebp_bytes=766 tiny_webp_to_cwebp_size_ratio=5.016 cwebp_rgb_psnr_db=42.604 cwebp_subprocess_ms=3.736
alpha-hard q90 megapixels_per_second=1.423 peak_heap_bytes_per_pixel=5.551 bytes=3842 rgb_psnr_db=44.627 cwebp_bytes=324 tiny_webp_to_cwebp_size_ratio=11.858 cwebp_rgb_psnr_db=12.565 cwebp_subprocess_ms=2.678
alpha-odd q90 megapixels_per_second=0.749 peak_heap_bytes_per_pixel=9.383 bytes=870 rgb_psnr_db=42.712 cwebp_bytes=306 tiny_webp_to_cwebp_size_ratio=2.843 cwebp_rgb_psnr_db=38.490 cwebp_subprocess_ms=2.816
photo-large q90 megapixels_per_second=1.083 peak_heap_bytes_per_pixel=5.104 bytes=433040 rgb_psnr_db=32.803 cwebp_bytes=245572 tiny_webp_to_cwebp_size_ratio=1.763 cwebp_rgb_psnr_db=32.091 cwebp_subprocess_ms=79.255
one-pixel q90 megapixels_per_second=0.006 peak_heap_bytes_per_pixel=1010.000 bytes=42 rgb_psnr_db=inf cwebp_bytes=46 tiny_webp_to_cwebp_size_ratio=0.913 cwebp_rgb_psnr_db=inf cwebp_subprocess_ms=2.591
single-column q90 megapixels_per_second=0.070 peak_heap_bytes_per_pixel=93.515 bytes=184 rgb_psnr_db=42.165 cwebp_bytes=130 tiny_webp_to_cwebp_size_ratio=1.415 cwebp_rgb_psnr_db=45.277 cwebp_subprocess_ms=2.488
single-row q90 megapixels_per_second=0.072 peak_heap_bytes_per_pixel=88.485 bytes=158 rgb_psnr_db=46.212 cwebp_bytes=124 tiny_webp_to_cwebp_size_ratio=1.274 cwebp_rgb_psnr_db=42.863 cwebp_subprocess_ms=2.373
odd-size q90 megapixels_per_second=0.650 peak_heap_bytes_per_pixel=10.803 bytes=704 rgb_psnr_db=44.864 cwebp_bytes=322 tiny_webp_to_cwebp_size_ratio=2.186 cwebp_rgb_psnr_db=42.663 cwebp_subprocess_ms=2.329
```
