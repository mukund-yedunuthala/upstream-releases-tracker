# Upstream Releases Tracker

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)

Track remote releases from GitHub/Forgejo. Built with Tauri, Rust and JS.

Hobby project built to figure out JSON I/O. Code refined using AI tools, any resemblance not intentional.

Frontend uses components from [Oat UI](https://github.com/knadh/oat) under [MIT License](https://raw.githubusercontent.com/knadh/oat/refs/heads/master/LICENSE)

## Installation
Download windows installer from [GitHub](https://github.com/mukund-yedunuthala/upstream-releases-tracker/releases/download/v3.2.4/Upstream.Releases.Tracker_3.2.4_x64_en-US.msi)

## Tech Stack
Frontend: Oat UI + Vite
Backend: Rust (Tauri 2.0)

## Building from source
#### Prerequisites
- Rust 
- Tauri (System requirements at [https://v2.tauri.app/start/prerequisites/](https://v2.tauri.app/start/prerequisites/))
- npm

#### Debug version
```
cargo tauri dev
```

#### Build
```
cargo tauri build
```
## License

```
    Upstream Releases Tracker is an application that can be used to track releases
    from remote code repositories, specifically GitHub and Forgejo API compatible instances.
    Copyright (C) 2026  Mukund Yedunuthala

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
```
