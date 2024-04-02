<div align="center">
  <img height="160px" src="/src/assets/vvconvert-logo.png">
  <h1>VVConvert</h1>
  <p align="center">H.266/VVC Video Converter</p>
  
  [![Static Badge](https://img.shields.io/badge/License-MIT-green)](/LICENSE.txt)
  [![Static Badge](https://img.shields.io/badge/Website-vvconvert.app-419fcc)](https://vvconvert.app)
  [![Dynamic JSON Badge](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fraw.githubusercontent.com%2Falectr0%2FVVConvert%2Fmain%2Fsrc-tauri%2Ftauri.conf.json&query=%24.package.version&label=Version&color=white)](https://github.com/alectr0/VVConvert/releases)



  
</div>

## Overview

VVConvert is a cross-platform H.266 (VVC) video encoder. It support most input formats with customized encoder parameters.

- Built-in FFmpeg resolver/checker.
- Drag-and-drop file input with codec parameter specification.
- Utilizes the vvenc library for efficient H.266/VVC encoding.
- Cross-platform support (Windows, Linux, Mac).

## Build
VVConvert leverages Rust & Clang for build process using Tauri. A working [Rust](https://www.rust-lang.org/) & [Clang](https://clang.llvm.org/) installation is required for building.

### How to build using Tauri & Rust?

To build VVConvert, perform the following steps:

```sh
# Clone the repository
git clone https://github.com/alectr0/VVConvert

# Navigate and Build
cd VVConvert/src-tauri
cargo build
```
Tauri also supports building using npm. More info found [here](https://tauri.app/v1/guides/building/windows). 

## Acknowledgments
[Tauri](https://tauri.app/) for an awesome cross-platform desktop framework.

[FFmpeg](https://ffmpeg.org/) for a great way to handle videos.

[Fraunhoffer HHI](https://www.hhi.fraunhofer.de/en/departments/vca/technologies-and-solutions/h266-vvc/fraunhofer-versatile-video-encoder-vvenc.html) for the development of the VVenC library.

## License
This project utilizes the VVenC library, which is licensed under the BSD 2-Clause License
Please refer to the [LICENSE.txt](/LICENSE.txt) file for license information.

Copyright (c) 2024, Alec Carter
