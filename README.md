# qobuz-player

## High resolution audio player backed by Qobuz

Powered by [Qobuz](https://www.qobuz.com). Requires a paid subscription. This does not allow you to listen for free.

The player includes a terminal ui, a web-ui and a RFID player. 
The web interface is ideal for a setup with a single board computer, e.g. Raspberry Pi, connected to the speaker system and controlled with a smartphone or tablet.

### Terminal UI
![TUI Screenshot](/assets/qobuz-player.png?raw=true)

#### Keyboard Shortcuts
Press <kbd>?</kbd> for an overview of all available keyboard shortcuts

### Web UI
<img src="/assets/qobuz-player-webui.png?raw=true" width="240">

### RFID player
![RFID player](/assets/rfid-player.gif?raw=true)

Read more [in the wiki](https://github.com/SofusA/qobuz-player/wiki/RFID-player)

## Player Features

- High resolution audio: Supports up to 24bit/192Khz (max quality Qobuz offers)
- MPRIS support (control via [playerctl](https://github.com/altdesktop/playerctl) or other D-Bus client)
- Gap-less playback
- Web UI 
- Terminal UI 

## Installation
### Fonts
The terminal ui needs a [nerdfont](https://www.nerdfonts.com/) to display icons for explicit and hi-resolution.

### Download Release

Download the tar.gz file for your supported OS from the releases page, extract the file and execute `qobuz-player` or copy it to your `$PATH`.

### Installation with cargo
```
cargo install --git https://github.com/SofusA/qobuz-player
```
### Installation from the aur (unofficial)
Users on arch-linux (based) systems can install it from the aur as [qobuz-player](https://aur.archlinux.org/packages/qobuz-player) ([-git](https://aur.archlinux.org/packages/qobuz-player-git))
### Build from source

Linux dependencies: `alsa-sys-devel`, `just`.
```
cargo build
```

## Development
1. Setup sqlx: `just create-env-file`. Only needed once. 
2. Init sqlite database: `init-database`.
3. For webui development in `qobuz-player-web`:
  - `npm i`. Install npm dependencies. 
  - `npm run watch`. Watch for style changes. 

## Get started

Run `qobuz-player --help` or `qobuz-player <subcommand> --help` to see all available options.

To get started:

```shell
qobuz-player config username {USERNAME}
qobuz-player config password {PASSWORD}
# or to get prompted for the password
qobuz-player config password

# open tui player
qobuz-player

# open player with web ui
qobuz-player open --web 
```

## Web UI

The player can start an embedded web interface. This is disabled by default and must be started with the `--web` argument. It also listens on `0.0.0.0:9888` by default. Change port with `--port` argument.

Go to `http://localhost:9888` to view the UI.

## Contribution
Feature requests, issues and contributions are very welcome.

## Credits
Qobuz-player started as a fork of [hifi.rs](https://github.com/iamdb/hifi.rs) but has since diverged. 
