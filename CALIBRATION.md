# Calibration

- Machine: Mac Studio (Mac15,14)
- CPU: Apple M3 Ultra, 28 cores
- Operating system: macOS 26.6.1 (25G76)

## 0.2.0

- Toolchain: rustup stable-aarch64-apple-darwin
- Rust compiler: rustc 1.98.1 (48a229cea 2026-09-01)
- libwebp: 1.6.0
- Build profile: release

Build and run with:

```sh
PATH=/Users/justinjones/.cargo/bin:/opt/homebrew/bin:$PATH cargo +stable build --release --bin tiny-webp
PATH=/Users/justinjones/.cargo/bin:/opt/homebrew/bin:$PATH cargo +stable run --release --example bench
```

The subprocess columns measure each command from launch through exit.
The commands alternate on the same PNG with `-quiet -q <q> <input> -o <output>`.
Each median uses five runs after one warm-up run per command.
The time ratio divides tiny-webp's median by cwebp's median.
Library throughput measures the encode call alone.

`peak_heap_bytes` measures heap growth during the encode call.
`memory_bound_held` compares that peak plus four bytes per pixel with
eight bytes per pixel plus 1 MiB.

```text
tiny-webp 0.1.1 on macos aarch64
flat q50 megapixels_per_second=14.397 peak_heap_bytes_per_pixel=68.204 bytes=58 rgb_psnr_db=42.902 peak_heap_bytes=69841 memory_bound_held=yes cwebp_bytes=70 tiny_webp_to_cwebp_size_ratio=0.829 cwebp_rgb_psnr_db=47.291 tiny_webp_subprocess_ms=2.004 cwebp_subprocess_ms=2.607 tiny_webp_to_cwebp_time_ratio=0.768
checker q50 megapixels_per_second=6.637 peak_heap_bytes_per_pixel=68.391 bytes=248 rgb_psnr_db=42.848 peak_heap_bytes=70032 memory_bound_held=yes cwebp_bytes=320 tiny_webp_to_cwebp_size_ratio=0.775 cwebp_rgb_psnr_db=41.726 tiny_webp_subprocess_ms=2.095 cwebp_subprocess_ms=2.740 tiny_webp_to_cwebp_time_ratio=0.765
diagonals q50 megapixels_per_second=11.415 peak_heap_bytes_per_pixel=32.663 bytes=336 rgb_psnr_db=40.359 peak_heap_bytes=75256 memory_bound_held=yes cwebp_bytes=290 tiny_webp_to_cwebp_size_ratio=1.159 cwebp_rgb_psnr_db=36.906 tiny_webp_subprocess_ms=2.114 cwebp_subprocess_ms=2.719 tiny_webp_to_cwebp_time_ratio=0.778
gradient q50 megapixels_per_second=7.628 peak_heap_bytes_per_pixel=25.495 bytes=320 rgb_psnr_db=39.933 peak_heap_bytes=78322 memory_bound_held=yes cwebp_bytes=192 tiny_webp_to_cwebp_size_ratio=1.667 cwebp_rgb_psnr_db=38.151 tiny_webp_subprocess_ms=2.286 cwebp_subprocess_ms=2.638 tiny_webp_to_cwebp_time_ratio=0.867
text-blocks q50 megapixels_per_second=7.126 peak_heap_bytes_per_pixel=25.742 bytes=1074 rgb_psnr_db=37.539 peak_heap_bytes=79079 memory_bound_held=yes cwebp_bytes=902 tiny_webp_to_cwebp_size_ratio=1.191 cwebp_rgb_psnr_db=34.881 tiny_webp_subprocess_ms=2.364 cwebp_subprocess_ms=2.703 tiny_webp_to_cwebp_time_ratio=0.875
noise q50 megapixels_per_second=3.803 peak_heap_bytes_per_pixel=26.103 bytes=2180 rgb_psnr_db=12.688 peak_heap_bytes=80187 memory_bound_held=yes cwebp_bytes=1850 tiny_webp_to_cwebp_size_ratio=1.178 cwebp_rgb_psnr_db=12.688 tiny_webp_subprocess_ms=2.686 cwebp_subprocess_ms=2.875 tiny_webp_to_cwebp_time_ratio=0.934
lowpass-noise q50 megapixels_per_second=5.884 peak_heap_bytes_per_pixel=25.597 bytes=628 rgb_psnr_db=29.302 peak_heap_bytes=78633 memory_bound_held=yes cwebp_bytes=454 tiny_webp_to_cwebp_size_ratio=1.383 cwebp_rgb_psnr_db=28.196 tiny_webp_subprocess_ms=2.503 cwebp_subprocess_ms=2.869 tiny_webp_to_cwebp_time_ratio=0.872
alpha-soft q50 megapixels_per_second=8.372 peak_heap_bytes_per_pixel=26.505 bytes=3420 rgb_psnr_db=39.933 peak_heap_bytes=81422 memory_bound_held=yes cwebp_bytes=662 tiny_webp_to_cwebp_size_ratio=5.166 cwebp_rgb_psnr_db=38.018 tiny_webp_subprocess_ms=2.501 cwebp_subprocess_ms=3.957 tiny_webp_to_cwebp_time_ratio=0.632
alpha-hard q50 megapixels_per_second=7.588 peak_heap_bytes_per_pixel=26.505 bytes=3420 rgb_psnr_db=39.933 peak_heap_bytes=81422 memory_bound_held=yes cwebp_bytes=236 tiny_webp_to_cwebp_size_ratio=14.492 cwebp_rgb_psnr_db=12.682 tiny_webp_subprocess_ms=2.410 cwebp_subprocess_ms=2.953 tiny_webp_to_cwebp_time_ratio=0.816
alpha-odd q50 megapixels_per_second=3.740 peak_heap_bytes_per_pixel=133.750 bytes=700 rgb_psnr_db=38.969 peak_heap_bytes=70486 memory_bound_held=yes cwebp_bytes=256 tiny_webp_to_cwebp_size_ratio=2.734 cwebp_rgb_psnr_db=37.166 tiny_webp_subprocess_ms=2.142 cwebp_subprocess_ms=3.109 tiny_webp_to_cwebp_time_ratio=0.689
photo-large q50 megapixels_per_second=6.315 peak_heap_bytes_per_pixel=4.260 bytes=138116 rgb_psnr_db=29.317 peak_heap_bytes=3350451 memory_bound_held=yes cwebp_bytes=88650 tiny_webp_to_cwebp_size_ratio=1.558 cwebp_rgb_psnr_db=28.109 tiny_webp_subprocess_ms=138.623 cwebp_subprocess_ms=63.344 tiny_webp_to_cwebp_time_ratio=2.188
one-pixel q50 megapixels_per_second=0.052 peak_heap_bytes_per_pixel=66742.000 bytes=44 rgb_psnr_db=99.000 peak_heap_bytes=66742 memory_bound_held=yes cwebp_bytes=44 tiny_webp_to_cwebp_size_ratio=1.000 cwebp_rgb_psnr_db=99.000 tiny_webp_subprocess_ms=1.966 cwebp_subprocess_ms=2.511 tiny_webp_to_cwebp_time_ratio=0.783
single-column q50 megapixels_per_second=0.382 peak_heap_bytes_per_pixel=2086.818 bytes=116 rgb_psnr_db=41.230 peak_heap_bytes=68865 memory_bound_held=yes cwebp_bytes=98 tiny_webp_to_cwebp_size_ratio=1.184 cwebp_rgb_psnr_db=37.526 tiny_webp_subprocess_ms=2.032 cwebp_subprocess_ms=2.518 tiny_webp_to_cwebp_time_ratio=0.807
single-row q50 megapixels_per_second=0.400 peak_heap_bytes_per_pixel=2087.485 bytes=114 rgb_psnr_db=43.478 peak_heap_bytes=68887 memory_bound_held=yes cwebp_bytes=96 tiny_webp_to_cwebp_size_ratio=1.188 cwebp_rgb_psnr_db=39.756 tiny_webp_subprocess_ms=2.016 cwebp_subprocess_ms=2.620 tiny_webp_to_cwebp_time_ratio=0.770
odd-size q50 megapixels_per_second=3.478 peak_heap_bytes_per_pixel=133.011 bytes=312 rgb_psnr_db=37.495 peak_heap_bytes=70097 memory_bound_held=yes cwebp_bytes=224 tiny_webp_to_cwebp_size_ratio=1.393 cwebp_rgb_psnr_db=33.143 tiny_webp_subprocess_ms=2.005 cwebp_subprocess_ms=2.643 tiny_webp_to_cwebp_time_ratio=0.758
flat q75 megapixels_per_second=25.441 peak_heap_bytes_per_pixel=68.206 bytes=58 rgb_psnr_db=45.121 peak_heap_bytes=69843 memory_bound_held=yes cwebp_bytes=74 tiny_webp_to_cwebp_size_ratio=0.784 cwebp_rgb_psnr_db=47.599 tiny_webp_subprocess_ms=2.067 cwebp_subprocess_ms=2.563 tiny_webp_to_cwebp_time_ratio=0.806
checker q75 megapixels_per_second=6.899 peak_heap_bytes_per_pixel=68.411 bytes=268 rgb_psnr_db=41.312 peak_heap_bytes=70053 memory_bound_held=yes cwebp_bytes=304 tiny_webp_to_cwebp_size_ratio=0.882 cwebp_rgb_psnr_db=46.325 tiny_webp_subprocess_ms=2.027 cwebp_subprocess_ms=2.698 tiny_webp_to_cwebp_time_ratio=0.751
diagonals q75 megapixels_per_second=12.732 peak_heap_bytes_per_pixel=32.714 bytes=452 rgb_psnr_db=44.757 peak_heap_bytes=75373 memory_bound_held=yes cwebp_bytes=328 tiny_webp_to_cwebp_size_ratio=1.378 cwebp_rgb_psnr_db=39.189 tiny_webp_subprocess_ms=2.189 cwebp_subprocess_ms=2.855 tiny_webp_to_cwebp_time_ratio=0.767
gradient q75 megapixels_per_second=7.762 peak_heap_bytes_per_pixel=25.501 bytes=334 rgb_psnr_db=40.930 peak_heap_bytes=78338 memory_bound_held=yes cwebp_bytes=220 tiny_webp_to_cwebp_size_ratio=1.518 cwebp_rgb_psnr_db=41.104 tiny_webp_subprocess_ms=2.503 cwebp_subprocess_ms=2.943 tiny_webp_to_cwebp_time_ratio=0.850
text-blocks q75 megapixels_per_second=6.405 peak_heap_bytes_per_pixel=25.812 bytes=1286 rgb_psnr_db=40.307 peak_heap_bytes=79294 memory_bound_held=yes cwebp_bytes=1022 tiny_webp_to_cwebp_size_ratio=1.258 cwebp_rgb_psnr_db=37.751 tiny_webp_subprocess_ms=2.406 cwebp_subprocess_ms=2.728 tiny_webp_to_cwebp_time_ratio=0.882
noise q75 megapixels_per_second=3.810 peak_heap_bytes_per_pixel=26.219 bytes=2536 rgb_psnr_db=12.761 peak_heap_bytes=80545 memory_bound_held=yes cwebp_bytes=2170 tiny_webp_to_cwebp_size_ratio=1.169 cwebp_rgb_psnr_db=12.748 tiny_webp_subprocess_ms=2.958 cwebp_subprocess_ms=3.186 tiny_webp_to_cwebp_time_ratio=0.928
lowpass-noise q75 megapixels_per_second=5.338 peak_heap_bytes_per_pixel=25.670 bytes=854 rgb_psnr_db=30.402 peak_heap_bytes=78857 memory_bound_held=yes cwebp_bytes=610 tiny_webp_to_cwebp_size_ratio=1.400 cwebp_rgb_psnr_db=29.753 tiny_webp_subprocess_ms=2.693 cwebp_subprocess_ms=2.980 tiny_webp_to_cwebp_time_ratio=0.904
alpha-soft q75 megapixels_per_second=7.740 peak_heap_bytes_per_pixel=26.510 bytes=3434 rgb_psnr_db=40.930 peak_heap_bytes=81438 memory_bound_held=yes cwebp_bytes=714 tiny_webp_to_cwebp_size_ratio=4.810 cwebp_rgb_psnr_db=40.037 tiny_webp_subprocess_ms=2.422 cwebp_subprocess_ms=3.988 tiny_webp_to_cwebp_time_ratio=0.607
alpha-hard q75 megapixels_per_second=7.499 peak_heap_bytes_per_pixel=26.510 bytes=3434 rgb_psnr_db=40.930 peak_heap_bytes=81438 memory_bound_held=yes cwebp_bytes=262 tiny_webp_to_cwebp_size_ratio=13.107 cwebp_rgb_psnr_db=12.583 tiny_webp_subprocess_ms=2.452 cwebp_subprocess_ms=2.986 tiny_webp_to_cwebp_time_ratio=0.821
alpha-odd q75 megapixels_per_second=3.770 peak_heap_bytes_per_pixel=133.713 bytes=682 rgb_psnr_db=40.374 peak_heap_bytes=70467 memory_bound_held=yes cwebp_bytes=266 tiny_webp_to_cwebp_size_ratio=2.564 cwebp_rgb_psnr_db=35.699 tiny_webp_subprocess_ms=2.129 cwebp_subprocess_ms=3.162 tiny_webp_to_cwebp_time_ratio=0.673
photo-large q75 megapixels_per_second=5.910 peak_heap_bytes_per_pixel=4.320 bytes=185324 rgb_psnr_db=30.471 peak_heap_bytes=3397666 memory_bound_held=yes cwebp_bytes=123630 tiny_webp_to_cwebp_size_ratio=1.499 cwebp_rgb_psnr_db=29.515 tiny_webp_subprocess_ms=146.956 cwebp_subprocess_ms=68.354 tiny_webp_to_cwebp_time_ratio=2.150
one-pixel q75 megapixels_per_second=0.039 peak_heap_bytes_per_pixel=66742.000 bytes=44 rgb_psnr_db=99.000 peak_heap_bytes=66742 memory_bound_held=yes cwebp_bytes=44 tiny_webp_to_cwebp_size_ratio=1.000 cwebp_rgb_psnr_db=99.000 tiny_webp_subprocess_ms=1.989 cwebp_subprocess_ms=2.704 tiny_webp_to_cwebp_time_ratio=0.735
single-column q75 megapixels_per_second=0.379 peak_heap_bytes_per_pixel=2087.061 bytes=126 rgb_psnr_db=40.284 peak_heap_bytes=68873 memory_bound_held=yes cwebp_bytes=110 tiny_webp_to_cwebp_size_ratio=1.145 cwebp_rgb_psnr_db=40.371 tiny_webp_subprocess_ms=2.056 cwebp_subprocess_ms=2.595 tiny_webp_to_cwebp_time_ratio=0.793
single-row q75 megapixels_per_second=0.428 peak_heap_bytes_per_pixel=2087.788 bytes=124 rgb_psnr_db=43.662 peak_heap_bytes=68897 memory_bound_held=yes cwebp_bytes=104 tiny_webp_to_cwebp_size_ratio=1.192 cwebp_rgb_psnr_db=42.646 tiny_webp_subprocess_ms=2.210 cwebp_subprocess_ms=2.644 tiny_webp_to_cwebp_time_ratio=0.836
odd-size q75 megapixels_per_second=3.241 peak_heap_bytes_per_pixel=133.186 bytes=400 rgb_psnr_db=40.070 peak_heap_bytes=70189 memory_bound_held=yes cwebp_bytes=252 tiny_webp_to_cwebp_size_ratio=1.587 cwebp_rgb_psnr_db=35.853 tiny_webp_subprocess_ms=2.115 cwebp_subprocess_ms=2.769 tiny_webp_to_cwebp_time_ratio=0.764
flat q90 megapixels_per_second=30.992 peak_heap_bytes_per_pixel=68.212 bytes=64 rgb_psnr_db=49.892 peak_heap_bytes=69849 memory_bound_held=yes cwebp_bytes=76 tiny_webp_to_cwebp_size_ratio=0.842 cwebp_rgb_psnr_db=48.936 tiny_webp_subprocess_ms=2.045 cwebp_subprocess_ms=2.533 tiny_webp_to_cwebp_time_ratio=0.808
checker q90 megapixels_per_second=5.238 peak_heap_bytes_per_pixel=68.672 bytes=534 rgb_psnr_db=57.442 peak_heap_bytes=70320 memory_bound_held=yes cwebp_bytes=354 tiny_webp_to_cwebp_size_ratio=1.508 cwebp_rgb_psnr_db=51.721 tiny_webp_subprocess_ms=2.223 cwebp_subprocess_ms=2.846 tiny_webp_to_cwebp_time_ratio=0.781
diagonals q90 megapixels_per_second=12.021 peak_heap_bytes_per_pixel=32.724 bytes=476 rgb_psnr_db=51.096 peak_heap_bytes=75396 memory_bound_held=yes cwebp_bytes=420 tiny_webp_to_cwebp_size_ratio=1.133 cwebp_rgb_psnr_db=46.547 tiny_webp_subprocess_ms=2.190 cwebp_subprocess_ms=2.824 tiny_webp_to_cwebp_time_ratio=0.776
gradient q90 megapixels_per_second=7.966 peak_heap_bytes_per_pixel=25.511 bytes=368 rgb_psnr_db=46.870 peak_heap_bytes=78370 memory_bound_held=yes cwebp_bytes=284 tiny_webp_to_cwebp_size_ratio=1.296 cwebp_rgb_psnr_db=43.377 tiny_webp_subprocess_ms=2.436 cwebp_subprocess_ms=2.763 tiny_webp_to_cwebp_time_ratio=0.882
text-blocks q90 megapixels_per_second=6.652 peak_heap_bytes_per_pixel=25.821 bytes=1316 rgb_psnr_db=47.102 peak_heap_bytes=79323 memory_bound_held=yes cwebp_bytes=1214 tiny_webp_to_cwebp_size_ratio=1.084 cwebp_rgb_psnr_db=44.473 tiny_webp_subprocess_ms=2.641 cwebp_subprocess_ms=2.995 tiny_webp_to_cwebp_time_ratio=0.882
noise q90 megapixels_per_second=3.169 peak_heap_bytes_per_pixel=26.483 bytes=3342 rgb_psnr_db=12.795 peak_heap_bytes=81356 memory_bound_held=yes cwebp_bytes=2950 tiny_webp_to_cwebp_size_ratio=1.133 cwebp_rgb_psnr_db=12.796 tiny_webp_subprocess_ms=3.039 cwebp_subprocess_ms=3.264 tiny_webp_to_cwebp_time_ratio=0.931
lowpass-noise q90 megapixels_per_second=4.666 peak_heap_bytes_per_pixel=25.900 bytes=1560 rgb_psnr_db=32.921 peak_heap_bytes=79566 memory_bound_held=yes cwebp_bytes=1142 tiny_webp_to_cwebp_size_ratio=1.366 cwebp_rgb_psnr_db=32.158 tiny_webp_subprocess_ms=2.857 cwebp_subprocess_ms=2.958 tiny_webp_to_cwebp_time_ratio=0.966
alpha-soft q90 megapixels_per_second=7.896 peak_heap_bytes_per_pixel=26.520 bytes=3468 rgb_psnr_db=46.870 peak_heap_bytes=81470 memory_bound_held=yes cwebp_bytes=766 tiny_webp_to_cwebp_size_ratio=4.527 cwebp_rgb_psnr_db=42.604 tiny_webp_subprocess_ms=2.450 cwebp_subprocess_ms=4.223 tiny_webp_to_cwebp_time_ratio=0.580
alpha-hard q90 megapixels_per_second=7.921 peak_heap_bytes_per_pixel=26.520 bytes=3468 rgb_psnr_db=46.870 peak_heap_bytes=81470 memory_bound_held=yes cwebp_bytes=324 tiny_webp_to_cwebp_size_ratio=10.704 cwebp_rgb_psnr_db=12.565 tiny_webp_subprocess_ms=2.432 cwebp_subprocess_ms=3.086 tiny_webp_to_cwebp_time_ratio=0.788
alpha-odd q90 megapixels_per_second=3.613 peak_heap_bytes_per_pixel=133.835 bytes=746 rgb_psnr_db=44.223 peak_heap_bytes=70531 memory_bound_held=yes cwebp_bytes=306 tiny_webp_to_cwebp_size_ratio=2.438 cwebp_rgb_psnr_db=38.490 tiny_webp_subprocess_ms=2.251 cwebp_subprocess_ms=3.299 tiny_webp_to_cwebp_time_ratio=0.682
photo-large q90 megapixels_per_second=4.767 peak_heap_bytes_per_pixel=4.566 bytes=335240 rgb_psnr_db=33.103 peak_heap_bytes=3591138 memory_bound_held=yes cwebp_bytes=245572 tiny_webp_to_cwebp_size_ratio=1.365 cwebp_rgb_psnr_db=32.091 tiny_webp_subprocess_ms=176.044 cwebp_subprocess_ms=79.929 tiny_webp_to_cwebp_time_ratio=2.203
one-pixel q90 megapixels_per_second=0.052 peak_heap_bytes_per_pixel=66743.000 bytes=44 rgb_psnr_db=99.000 peak_heap_bytes=66743 memory_bound_held=yes cwebp_bytes=46 tiny_webp_to_cwebp_size_ratio=0.957 cwebp_rgb_psnr_db=99.000 tiny_webp_subprocess_ms=1.954 cwebp_subprocess_ms=2.671 tiny_webp_to_cwebp_time_ratio=0.732
single-column q90 megapixels_per_second=0.373 peak_heap_bytes_per_pixel=2087.879 bytes=152 rgb_psnr_db=42.915 peak_heap_bytes=68900 memory_bound_held=yes cwebp_bytes=130 tiny_webp_to_cwebp_size_ratio=1.169 cwebp_rgb_psnr_db=45.277 tiny_webp_subprocess_ms=2.338 cwebp_subprocess_ms=2.903 tiny_webp_to_cwebp_time_ratio=0.805
single-row q90 megapixels_per_second=0.431 peak_heap_bytes_per_pixel=2088.879 bytes=160 rgb_psnr_db=46.473 peak_heap_bytes=68933 memory_bound_held=yes cwebp_bytes=124 tiny_webp_to_cwebp_size_ratio=1.290 cwebp_rgb_psnr_db=42.863 tiny_webp_subprocess_ms=2.103 cwebp_subprocess_ms=2.698 tiny_webp_to_cwebp_time_ratio=0.780
odd-size q90 megapixels_per_second=3.587 peak_heap_bytes_per_pixel=133.167 bytes=392 rgb_psnr_db=46.558 peak_heap_bytes=70179 memory_bound_held=yes cwebp_bytes=322 tiny_webp_to_cwebp_size_ratio=1.217 cwebp_rgb_psnr_db=42.663 tiny_webp_subprocess_ms=2.162 cwebp_subprocess_ms=2.635 tiny_webp_to_cwebp_time_ratio=0.820
```

## 0.1.0

- Rust compiler: rustc 1.98.0 (88d9e12ae 2026-08-18) (Homebrew)
- libwebp: 1.6.0
- Build profile: release

Command: `PATH=/opt/homebrew/bin:$PATH cargo run --release --example bench`

```text
tiny-webp 0.1.0 on macos aarch64
flat q50 megapixels_per_second=49.349 peak_heap_bytes_per_pixel=4.265 bytes=56 rgb_psnr_db=48.131 cwebp_bytes=70 tiny_webp_to_cwebp_size_ratio=0.800 cwebp_rgb_psnr_db=47.291 cwebp_subprocess_ms=2.910
checker q50 megapixels_per_second=21.558 peak_heap_bytes_per_pixel=5.526 bytes=702 rgb_psnr_db=42.433 cwebp_bytes=320 tiny_webp_to_cwebp_size_ratio=2.194 cwebp_rgb_psnr_db=41.726 cwebp_subprocess_ms=2.830
gradient q50 megapixels_per_second=43.600 peak_heap_bytes_per_pixel=4.288 bytes=352 rgb_psnr_db=35.329 cwebp_bytes=192 tiny_webp_to_cwebp_size_ratio=1.833 cwebp_rgb_psnr_db=38.151 cwebp_subprocess_ms=2.588
text-blocks q50 megapixels_per_second=20.600 peak_heap_bytes_per_pixel=5.297 bytes=1902 rgb_psnr_db=35.797 cwebp_bytes=902 tiny_webp_to_cwebp_size_ratio=2.109 cwebp_rgb_psnr_db=34.881 cwebp_subprocess_ms=2.471
noise q50 megapixels_per_second=12.276 peak_heap_bytes_per_pixel=6.041 bytes=3044 rgb_psnr_db=12.684 cwebp_bytes=1850 tiny_webp_to_cwebp_size_ratio=1.645 cwebp_rgb_psnr_db=12.688 cwebp_subprocess_ms=2.843
lowpass-noise q50 megapixels_per_second=30.991 peak_heap_bytes_per_pixel=4.447 bytes=596 rgb_psnr_db=28.934 cwebp_bytes=454 tiny_webp_to_cwebp_size_ratio=1.313 cwebp_rgb_psnr_db=28.196 cwebp_subprocess_ms=2.668
alpha-soft q50 megapixels_per_second=42.372 peak_heap_bytes_per_pixel=5.297 bytes=3452 rgb_psnr_db=35.329 cwebp_bytes=662 tiny_webp_to_cwebp_size_ratio=5.215 cwebp_rgb_psnr_db=38.018 cwebp_subprocess_ms=3.533
alpha-hard q50 megapixels_per_second=42.421 peak_heap_bytes_per_pixel=5.297 bytes=3452 rgb_psnr_db=35.329 cwebp_bytes=236 tiny_webp_to_cwebp_size_ratio=14.627 cwebp_rgb_psnr_db=12.682 cwebp_subprocess_ms=2.520
alpha-odd q50 megapixels_per_second=16.092 peak_heap_bytes_per_pixel=8.827 bytes=724 rgb_psnr_db=38.550 cwebp_bytes=256 tiny_webp_to_cwebp_size_ratio=2.828 cwebp_rgb_psnr_db=37.166 cwebp_subprocess_ms=2.591
photo-large q50 megapixels_per_second=43.719 peak_heap_bytes_per_pixel=4.341 bytes=132954 rgb_psnr_db=28.907 cwebp_bytes=88650 tiny_webp_to_cwebp_size_ratio=1.500 cwebp_rgb_psnr_db=28.109 cwebp_subprocess_ms=59.347
one-pixel q50 megapixels_per_second=0.085 peak_heap_bytes_per_pixel=1009.000 bytes=42 rgb_psnr_db=inf cwebp_bytes=44 tiny_webp_to_cwebp_size_ratio=0.955 cwebp_rgb_psnr_db=inf cwebp_subprocess_ms=2.549
single-column q50 megapixels_per_second=1.417 peak_heap_bytes_per_pixel=85.152 bytes=112 rgb_psnr_db=35.939 cwebp_bytes=98 tiny_webp_to_cwebp_size_ratio=1.143 cwebp_rgb_psnr_db=37.526 cwebp_subprocess_ms=2.345
single-row q50 megapixels_per_second=1.890 peak_heap_bytes_per_pixel=83.000 bytes=100 rgb_psnr_db=38.351 cwebp_bytes=96 tiny_webp_to_cwebp_size_ratio=1.042 cwebp_rgb_psnr_db=39.756 cwebp_subprocess_ms=2.264
odd-size q50 megapixels_per_second=12.046 peak_heap_bytes_per_pixel=9.127 bytes=526 rgb_psnr_db=35.583 cwebp_bytes=224 tiny_webp_to_cwebp_size_ratio=2.348 cwebp_rgb_psnr_db=33.143 cwebp_subprocess_ms=2.312
flat q75 megapixels_per_second=57.287 peak_heap_bytes_per_pixel=4.269 bytes=58 rgb_psnr_db=45.121 cwebp_bytes=74 tiny_webp_to_cwebp_size_ratio=0.784 cwebp_rgb_psnr_db=47.599 cwebp_subprocess_ms=2.199
checker q75 megapixels_per_second=23.585 peak_heap_bytes_per_pixel=5.574 bytes=726 rgb_psnr_db=41.312 cwebp_bytes=304 tiny_webp_to_cwebp_size_ratio=2.388 cwebp_rgb_psnr_db=46.325 cwebp_subprocess_ms=2.174
gradient q75 megapixels_per_second=45.121 peak_heap_bytes_per_pixel=4.318 bytes=398 rgb_psnr_db=38.508 cwebp_bytes=220 tiny_webp_to_cwebp_size_ratio=1.809 cwebp_rgb_psnr_db=41.104 cwebp_subprocess_ms=2.295
text-blocks q75 megapixels_per_second=21.210 peak_heap_bytes_per_pixel=5.486 bytes=2192 rgb_psnr_db=39.309 cwebp_bytes=1022 tiny_webp_to_cwebp_size_ratio=2.145 cwebp_rgb_psnr_db=37.751 cwebp_subprocess_ms=2.372
noise q75 megapixels_per_second=10.614 peak_heap_bytes_per_pixel=7.581 bytes=3874 rgb_psnr_db=12.753 cwebp_bytes=2170 tiny_webp_to_cwebp_size_ratio=1.785 cwebp_rgb_psnr_db=12.748 cwebp_subprocess_ms=2.469
lowpass-noise q75 megapixels_per_second=26.646 peak_heap_bytes_per_pixel=4.608 bytes=844 rgb_psnr_db=30.470 cwebp_bytes=610 tiny_webp_to_cwebp_size_ratio=1.384 cwebp_rgb_psnr_db=29.753 cwebp_subprocess_ms=2.387
alpha-soft q75 megapixels_per_second=40.621 peak_heap_bytes_per_pixel=5.327 bytes=3498 rgb_psnr_db=38.508 cwebp_bytes=714 tiny_webp_to_cwebp_size_ratio=4.899 cwebp_rgb_psnr_db=40.037 cwebp_subprocess_ms=3.508
alpha-hard q75 megapixels_per_second=41.772 peak_heap_bytes_per_pixel=5.327 bytes=3498 rgb_psnr_db=38.508 cwebp_bytes=262 tiny_webp_to_cwebp_size_ratio=13.351 cwebp_rgb_psnr_db=12.583 cwebp_subprocess_ms=2.404
alpha-odd q75 megapixels_per_second=15.538 peak_heap_bytes_per_pixel=8.918 bytes=748 rgb_psnr_db=38.405 cwebp_bytes=266 tiny_webp_to_cwebp_size_ratio=2.812 cwebp_rgb_psnr_db=35.699 cwebp_subprocess_ms=2.534
photo-large q75 megapixels_per_second=35.050 peak_heap_bytes_per_pixel=4.504 bytes=196866 rgb_psnr_db=30.524 cwebp_bytes=123630 tiny_webp_to_cwebp_size_ratio=1.592 cwebp_rgb_psnr_db=29.515 cwebp_subprocess_ms=62.936
one-pixel q75 megapixels_per_second=0.106 peak_heap_bytes_per_pixel=1009.000 bytes=42 rgb_psnr_db=inf cwebp_bytes=44 tiny_webp_to_cwebp_size_ratio=0.955 cwebp_rgb_psnr_db=inf cwebp_subprocess_ms=2.633
single-column q75 megapixels_per_second=1.472 peak_heap_bytes_per_pixel=86.152 bytes=128 rgb_psnr_db=39.018 cwebp_bytes=110 tiny_webp_to_cwebp_size_ratio=1.164 cwebp_rgb_psnr_db=40.371 cwebp_subprocess_ms=2.233
single-row q75 megapixels_per_second=1.678 peak_heap_bytes_per_pixel=85.576 bytes=110 rgb_psnr_db=40.483 cwebp_bytes=104 tiny_webp_to_cwebp_size_ratio=1.058 cwebp_rgb_psnr_db=42.646 cwebp_subprocess_ms=2.362
odd-size q75 megapixels_per_second=10.317 peak_heap_bytes_per_pixel=10.349 bytes=584 rgb_psnr_db=38.787 cwebp_bytes=252 tiny_webp_to_cwebp_size_ratio=2.317 cwebp_rgb_psnr_db=35.853 cwebp_subprocess_ms=2.310
flat q90 megapixels_per_second=56.110 peak_heap_bytes_per_pixel=4.288 bytes=68 rgb_psnr_db=46.618 cwebp_bytes=76 tiny_webp_to_cwebp_size_ratio=0.895 cwebp_rgb_psnr_db=48.936 cwebp_subprocess_ms=2.360
checker q90 megapixels_per_second=22.161 peak_heap_bytes_per_pixel=5.730 bytes=806 rgb_psnr_db=inf cwebp_bytes=354 tiny_webp_to_cwebp_size_ratio=2.277 cwebp_rgb_psnr_db=51.721 cwebp_subprocess_ms=2.505
gradient q90 megapixels_per_second=34.244 peak_heap_bytes_per_pixel=4.542 bytes=742 rgb_psnr_db=44.627 cwebp_bytes=284 tiny_webp_to_cwebp_size_ratio=2.613 cwebp_rgb_psnr_db=43.377 cwebp_subprocess_ms=2.331
text-blocks q90 megapixels_per_second=18.026 peak_heap_bytes_per_pixel=5.906 bytes=2836 rgb_psnr_db=45.478 cwebp_bytes=1214 tiny_webp_to_cwebp_size_ratio=2.336 cwebp_rgb_psnr_db=44.473 cwebp_subprocess_ms=2.372
noise q90 megapixels_per_second=8.352 peak_heap_bytes_per_pixel=11.159 bytes=6298 rgb_psnr_db=12.792 cwebp_bytes=2950 tiny_webp_to_cwebp_size_ratio=2.135 cwebp_rgb_psnr_db=12.796 cwebp_subprocess_ms=2.662
lowpass-noise q90 megapixels_per_second=16.902 peak_heap_bytes_per_pixel=5.231 bytes=1800 rgb_psnr_db=32.642 cwebp_bytes=1142 tiny_webp_to_cwebp_size_ratio=1.576 cwebp_rgb_psnr_db=32.158 cwebp_subprocess_ms=2.505
alpha-soft q90 megapixels_per_second=33.945 peak_heap_bytes_per_pixel=5.551 bytes=3842 rgb_psnr_db=44.627 cwebp_bytes=766 tiny_webp_to_cwebp_size_ratio=5.016 cwebp_rgb_psnr_db=42.604 cwebp_subprocess_ms=3.570
alpha-hard q90 megapixels_per_second=36.212 peak_heap_bytes_per_pixel=5.551 bytes=3842 rgb_psnr_db=44.627 cwebp_bytes=324 tiny_webp_to_cwebp_size_ratio=11.858 cwebp_rgb_psnr_db=12.565 cwebp_subprocess_ms=2.516
alpha-odd q90 megapixels_per_second=14.085 peak_heap_bytes_per_pixel=9.383 bytes=870 rgb_psnr_db=42.712 cwebp_bytes=306 tiny_webp_to_cwebp_size_ratio=2.843 cwebp_rgb_psnr_db=38.490 cwebp_subprocess_ms=2.572
photo-large q90 megapixels_per_second=22.800 peak_heap_bytes_per_pixel=5.104 bytes=433040 rgb_psnr_db=32.803 cwebp_bytes=245572 tiny_webp_to_cwebp_size_ratio=1.763 cwebp_rgb_psnr_db=32.091 cwebp_subprocess_ms=72.778
one-pixel q90 megapixels_per_second=0.108 peak_heap_bytes_per_pixel=1010.000 bytes=42 rgb_psnr_db=inf cwebp_bytes=46 tiny_webp_to_cwebp_size_ratio=0.913 cwebp_rgb_psnr_db=inf cwebp_subprocess_ms=2.624
single-column q90 megapixels_per_second=1.122 peak_heap_bytes_per_pixel=93.515 bytes=184 rgb_psnr_db=42.165 cwebp_bytes=130 tiny_webp_to_cwebp_size_ratio=1.415 cwebp_rgb_psnr_db=45.277 cwebp_subprocess_ms=2.480
single-row q90 megapixels_per_second=1.711 peak_heap_bytes_per_pixel=88.485 bytes=158 rgb_psnr_db=46.212 cwebp_bytes=124 tiny_webp_to_cwebp_size_ratio=1.274 cwebp_rgb_psnr_db=42.863 cwebp_subprocess_ms=2.204
odd-size q90 megapixels_per_second=10.728 peak_heap_bytes_per_pixel=10.803 bytes=704 rgb_psnr_db=44.864 cwebp_bytes=322 tiny_webp_to_cwebp_size_ratio=2.186 cwebp_rgb_psnr_db=42.663 cwebp_subprocess_ms=2.340
```
