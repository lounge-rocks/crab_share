# S3 crab_share

A super simple application to upload files to an S3 bucket and generate a shareable link.

## Usage

```bash
sharepy <file>
```

Options:

| Option           | Default      | Description                      |
| ---------------- | -------      | -------------------------------- |
| -e, --expires    | 7d           | The time until the link expires. |
| -b, --bucket     |              | The bucket to upload to.         |
| -u, --url        |              | The S3 url.                      |
| -r, --region     | eu-central-1 | The S3 region.                   |
| -a, --access-key |              | The S3 access key.               |
| -s, --secret-key |              | The S3 secret key.               |
| -t, --token      |              | The S3 session token.            |

## Setup

There are three ways to configure the application. Either by passing options, setting the environment variables, or by creating a config file containing the credentials.
The default options are overwritten by the config file, which are overwritten by the environment variables, which are overwritten by the passed options.

### Environment variables

```bash
export S3_URL=
export S3_ACCESS_KEY=
export S3_SECRET_KEY=
export S3_SESSION_TOKEN=
export S3_SECURITY_TOKEN=
export S3_PROFILE=
export S3_EXPIRES=
export S3_BUCKET=
export S3_PATH=
export S3_REGION=
```

### Token file

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
    "path": "auto",
    "sessionToken": "sessionToken",
    "securityToken": "securityToken",
    "profile": "profile"
}
```

### Config file

```bash
touch ~/.aws/crab_share.json
vim ~/.aws/crab_share.json
```

The file should have the following format:

```json
{
    "bucket": "your-bucket-name",
    "region": "eu-central-1",
    "url": "https://s3.domain.com",
    "expires": "7d"
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
