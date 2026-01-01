<a name="readme-top"></a>
<!-- PROJECT LOGO -->
<br />
<div align="center">
  <a href="https://github.com/Niatsuna/Kohaku">
    <img src="data/banner.png" alt="Logo" height="200">
  </a>

<h3 align="center">Kohaku</h3>

  <p align="center">
    A Discord bot with Rust backend that scrapes, manages and analyzes data, providing information to Discord servers through a REST API.
  </p>
</div>
<!-- Top Badges -->
<div align="center">

  ![kohaku version](https://img.shields.io/badge/version-pre3.0.0-orange?style=flat)
  ![server tests](https://img.shields.io/github/actions/workflow/status/Niatsuna/Kohaku/rust-ci.yml?label=Server%20Tests&branch=main)
  ![client tests](https://img.shields.io/github/actions/workflow/status/Niatsuna/Kohaku/python-ci.yml?label=Client%20Tests&branch=main)

  > ⚠️ The current main branch is not a full release yet but shows minimal core features ⚠️

  _please stay tuned for release `3.0.0`_

</div>


<!-- OVERVIEW -->
## Overview
Kohaku is an autonomous data pipeline combining a Rust backend with a Python Discord bot client.
The backend handles the complete data lifecycle with scraping external sources, persistent storage, automatic updates and API delivery, while the Python client provides intuitive Discord-native interactions.

Designed as a personal hobby project, it emphasizes self-sustaining architecture, where users always receive current data without backend intervention.
The dual-language approach leverages Rust's performance and reliability for heavy data operations alongside Python's rich Discord ecosystem for seamless bot functionality.

### Built With

![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![Actix Web](https://img.shields.io/badge/Actix%20Web-000000?style=for-the-badge&logo=rust&logoColor=white)
![Diesel](https://img.shields.io/badge/Diesel-000000?style=for-the-badge&logo=rust&logoColor=white)
![Python](https://img.shields.io/badge/Python_3.12-3776AB?style=for-the-badge&logo=python&logoColor=white)
![Disnake](https://img.shields.io/badge/Disnake-5865F2?style=for-the-badge&logo=discord&logoColor=white)

<i>For the full crate & package list, please see [Cargo.toml](/server/Cargo.toml) and [requirements.txt](/client/requirements.txt) respectively!</i>

### Features
> TODO:

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- GETTING STARTED -->
## Getting Started
### Prerequisites
Local Setup (Non-Docker):
- `Rust >= 1.90.0`
- `Python >= 3.12.0`

Configuration:
- Kohaku's configuration is based on an `.env` file.
  Therefore before deploying Kohaku in any way, fill out `.env.sample` and rename it to `.env`!
- Default aspects:
  - Database: PostgreSQL 17
  - Prefix: `-`
  - Logging Level (both) : INFO

### Deployment
The easiest way to deploy Kohaku is by using the provided `docker-compose.yml` while executing `docker compose up`. This will deploy server, client and database (by default a postgreSQL 17 database).

For development we recommend a local setup of server and client:
#### Rust Backend Server
Cargo is the Rust packaging manager, resulting in no further installation needed, as all packages are listed in the `Cargo.toml`.
```sh
# Move to right directory
cd server

# Start Kohaku Backend
cargo run
```

#### Python Discord Client
The packages for the client are listed in `requirements.txt`.
```sh
# Move to right directory
cd client

# Install required packages
pip install -r requirements.txt 

# Start Kohaku Client
python main.py
```

#### Developer Setup
While the prior sections cover simple deployment installations, some tools could be helpful during further development:
- [diesel cli](https://diesel.rs/guides/getting-started) - Database migrations (Backend), can be installed via `cargo install diesel_cli`
- Linter & Checker (Client) - Better code quality of client, pre-configured, can be installed via `pip install -r requirements-dev.txt`

Additionally, Makefiles present shortcut executions for formatting and checking both server and client.
<details>
<summary> Makefile Usage </summary>

The root makefile harbors six commands:
```sh
make fmt              # Formats server & client
make fmt-server       # Formats only server
make fmt-client       # Formats only client

make check            # Checks server & client
make check-server     # Checks only server
make check-client     # Checks only client
```
The makefiles in each directory (`server/` and `client/`) feature delegation of these commands.
Thus, `make fmt` in `server/` is equal to `make fmt-server` in `server/` and `make fmt-client` is disabled. The same applies to the other commands and to `client/`s makefile.

</details>

<!-- USAGE EXAMPLES -->
## Usage
After deployment, the backend runs autonomously, and run scheduled tasks to guarantee the freshness of stored data.

The client can be accessed via commands on any Discord server where it's present.

> TODO: Include example screenshot

<!-- DOCUMENTATION -->
### Documentation
> TODO: Provide explicit information of data flows and ideas deployed in this project

<!-- LICENSE -->
## License

Distributed under the MIT License. See  [`LICENSE`](/LICENSE) for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[repo-url]: https://github.com/Niatsuna/Kohaku