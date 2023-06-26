# S3 crab_share

A super simple application to upload files to an S3 bucket and generate a shareable link.

## Usage

```bash
sharepy <file>
```

Options:

| Option        | Default | Description                      |
| ------------- | ------- | -------------------------------- |
| -e, --expires | 7d      | The time until the link expires. |

## Setup

There are two ways to configure the application. Either by setting the environment variables, or by creating a config file containing the credentials.

### Environment variables

```bash
export S3_URL=
export S3_ACCESS_KEY=
export S3_SECRET_KEY=
```

### Config file

```bash
mkdir ~/.aws
touch ~/.aws/credentials.json
vim ~/.aws/credentials.json
```

The file should have the following format:

```json
{
    "url": "https://s3.domain.com",
    "accessKey": "accessKey",
    "secretKey": "secretKey",
    "api": "s3v4",
    "path": "auto"
}
```

## Installation

### Using the Nix package manager

This repository contains a `flake.nix` file which can be used to build the application using the Nix package manager.

### Using homebrew

To install the application using homebrew, run:

```bash
# Add the tap to homebrew
brew tap --force-auto-update lounge-rocks/crab_share https://github.com/lounge-rocks/crab_share

# Install the application
brew install crab_share

# Install the application from main branch
brew install --HEAD crab_share 
```

In case you want to remove the application, run:

```bash
brew uninstall crab_share
brew untap lounge-rocks/crab_share
brew autoremove
```

### Using cargo

Make sure you have cargo installed.
To do this on MacOS, run:

```bash
brew install rustup
rustup-init
```

Then clone the repository and build the binary.

```bash
git clone https://github.com/lounge-rocks/crab_share.git
cd crab_share
cargo install --path .
```

It's also possible to just build the binary without installing it.

```bash
cargo build --release
```
