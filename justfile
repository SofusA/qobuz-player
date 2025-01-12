detected_target := if os() == "linux" {
  if arch() == "x86_64" {
    "x86_64-unknown-linux-gnu"
  } else if arch() == "aarch64" {
    "aarch64-unknown-linux-gnu"
  } else {
    error("unknown os and/or arch")
  }
} else {
  error("unsupported os and/or arch")
}

default:
  @just --list

build-player target=detected_target $DATABASE_URL="sqlite:///tmp/data.db":
  just install-deps {{target}}
  just add-target {{target}}
  just install-toolchain "stable" {{target}}
  just install-sqlx
  just reset-database
  just build-tailwind
  just build-bin {{target}}

build-bin target=detected_target:
  @echo Building for target {{target}}
  cargo build --release --target={{target}}

build-bin-debug target=detected_target:
  cargo build --target={{target}}

install-deps target=detected_target:
  #!/usr/bin/env sh
  if ! just check-deps; then
    {{ if target == "x86_64-unknown-linux-gnu" { "just install-deps-linux-x86_64" } else if target == "aarch64-unknown-linux-gnu" { "just install-deps-linux-aarch64" } else { error("unsupported arch") } }}
    echo "Dependencies installed successfully for {{target}}"
  fi

check-rustup:
  #!/usr/bin/env sh
  if ! [ -x "$(command -v rustup)" ]; then
    echo 'Error: rustup is not installed.' >&2
    exit 1
  fi

add-target target=detected_target:
  rustup target add {{target}}

install-toolchain kind="stable" target=detected_target:
  rustup toolchain install {{kind}}-{{target}}

install-sqlx:
  cargo install sqlx-cli --force

reset-database:
  touch $(echo $DATABASE_URL | sed -e "s/sqlite:\/\///g") && cargo sqlx database reset --source {{invocation_directory()}}/qobuz-player-player/migrations -y

build-tailwind:
  cd qobuz-player-web && npm install && npm run build

check-deps:
  #!/usr/bin/env sh
  echo "Checking for required dependecies..."
  if ! [ -x "$(command -v pkg-config)" ]; then
    echo 'pkg-config not installed'
    exit 1
  fi

  if $(pkg-config --atleast-version "1.18" gstreamer-1.0); then
    echo "Dependencies found!"
  else
    exit 1
  fi

[linux]
install-deps-linux-x86_64:
  #!/usr/bin/env sh
  echo Installing dependencies for x86_64-unknown-linux-gnu
  sudo_cmd=''

  if [ -x "$(command -v sudo)" ]; then
    sudo_cmd='sudo '
  fi

  if [ -x "$(command -v apt-get)" ]; then
    eval $sudo_cmd apt-get update && DEBIAN_FRONTEND=noninteractive $sudo_cmd apt-get install -qq libunwind-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev nodejs npm
  elif [ -x "$(command -v pacman)" ]; then
    eval $sudo_cmd pacman -S gstreamer gst-plugins-base-libs nodejs
  elif [ -x "$(command -v dnf)" ]; then
    eval $sudo_cmd dnf install gstreamer1-devel gstreamer1-plugins-base-devel nodejs18
  else
    echo "distro not supported for x86_64-unknown-linux-gnu"
    exit 1
  fi

[linux]
install-deps-linux-aarch64:
  #!/usr/bin/env sh
  echo Installing dependencies for aarch64-unknown-linux-gnu
  sudo_cmd=''

  if [ -x "$(command -v sudo)" ]; then
    sudo_cmd='sudo '
  fi

  if [ -x "$(command -v apt-get)" ]; then
    eval $sudo_cmd dpkg --add-architecture arm64
    eval $sudo_cmd apt-get update && DEBIAN_FRONTEND=noninteractive apt-get -qq install curl libgstreamer1.0-dev:arm64 g++-aarch64-linux-gnu libc6-dev-arm64-cross libglib2.0-dev:arm64 nodejs npm
  else
    echo "distro not supported for aarch64-unknown-linux-gnu"
    exit 1
  fi
