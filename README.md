# qobuz-player

## High resolution audio player backed by Qobuz

Powered by [Qobuz](https://www.qobuz.com). Requires a paid subscription. This does not allow you to listen for free.

The player includes a terminal ui, a webui and a RFID player. 
The web interface is ideal for a setup with a single board computer, e.g. Raspberry Pi, connected to the speaker system and controlled with a smartphone or tablet.

### Terminal UI
![TUI Screenshot](/assets/qobuz-player.png?raw=true)

### Web UI
<img src="/assets/qobuz-player-webui.png?raw=true" width="240">

### RFID player
![RFID player](/assets/rfid-player.gif?raw=true)

Read more [in the wiki](https://github.com/SofusA/qobuz-player/wiki/RFID-player)

## Player Features

- High resolution audio: Supports up to 24bit/192Khz (max quality Qobuz offers)
- MPRIS support (control via [playerctl](https://github.com/altdesktop/playerctl) or other D-Bus client)
- Gapless playback
- Web UI 
- Terminal UI 

## Installation

### Download Release

Download the tar.gz file for your supported OS from the releases page, extract the file and execute `qobuz-player` or copy it to your `$PATH`.

## Get started

Run `qobuz-player --help` or `qobuz-player <subcommand> --help` to see all available options.

To get started:

```shell
qobuz-player config username # enter username at prompt
qobuz-player config password # enter password at prompt

# open player
qobuz-player open

# open player with web ui
qobuz-player --web open
```

## Web UI

The player can start an embedded web interface. This is disabled by default and must be started with the `--web` argument. It also listens on `0.0.0.0:9888` by default,
but an inteface can be specified with the `--interface` argument.

Go to `http://<ip>:9888` to view the UI.

## Contribution
Feature requests, issues and contributions are very welcome.

## Credits
Qobuz-player started as a fork of [hifi.rs](https://github.com/iamdb/hifi.rs) but has since diverged. 
