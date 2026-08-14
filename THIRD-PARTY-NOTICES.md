# Third-Party Notices

Olé is licensed under the Apache License, Version 2.0 (see `LICENSE`).

This application bundles, links against, or otherwise incorporates the
third-party components listed below. Each remains under its own license, and
those licenses are fully compatible with redistribution of Olé under
Apache-2.0. Where a component is redistributed in binary form, its required
copyright and permission notices are reproduced here.

> Maintainer note: when shipping a release, ensure the upstream `LICENSE`
> file for each bundled native binary (whisper.cpp, sherpa-onnx, and any
> downloaded model) is included alongside the distributed binary, as required
> by their licenses.

---

## Bundled native engines

### whisper.cpp
- Purpose: local speech-to-text inference engine.
- Homepage: https://github.com/ggml-org/whisper.cpp
- License: MIT

```
MIT License

Copyright (c) 2023-2024 The ggml authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### sherpa-onnx
- Purpose: on-device speech recognition runtime (ONNX-based models).
- Homepage: https://github.com/k2-fsa/sherpa-onnx
- License: Apache-2.0
- Copyright 2022-2024 Xiaomi Corporation and the k2-fsa authors.
- Licensed under the Apache License, Version 2.0. A copy of the Apache-2.0
  license is provided in this repository's `LICENSE` file and at
  http://www.apache.org/licenses/LICENSE-2.0

---

## Speech-to-text models

Whisper models (`ggml-*.bin`) are derived from OpenAI's Whisper.
- Homepage: https://github.com/openai/whisper
- License: MIT
- Models are downloaded by the user at runtime and are not redistributed as
  part of this source repository.

---

## Application frameworks and libraries

The desktop application is built with the following major open-source projects,
each under a permissive license (MIT or Apache-2.0):

- Tauri — MIT / Apache-2.0 — https://github.com/tauri-apps/tauri
- React — MIT — https://github.com/facebook/react
- Vite — MIT — https://github.com/vitejs/vite
- Zustand — MIT — https://github.com/pmndrs/zustand
- lucide-react — ISC — https://github.com/lucide-icons/lucide

Rust crate dependencies (see `src-tauri/Cargo.toml`) and npm dependencies
(see `package.json`) are licensed under their respective terms, predominantly
MIT and Apache-2.0. A complete, generated dependency license report can be
produced with `cargo about` (Rust) and `license-checker` (npm) if a full
manifest is required for a distribution.
