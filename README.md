# qobuz-player

### High resolution audio player backed by Qobuz

Powered by [Qobuz](https://www.qobuz.com). Requires a paid subscription. This does not allow you to listen for free.

The player includes a terminal ui and a webui. 
The web interface is ideal for a setup with a single board conputer, e.g. Raspberry Pi, connected to the speaker system and controlled with a smartphone or tablet.

## Terminal UI screenshot
![TUI Screenshot](/qobuz-player.png?raw=true)

## Web UI screenshot
<img src="/qobuz-player-webui.png?raw=true" width="240">

## Player Features

- [GStreamer](https://gstreamer.freedesktop.org/)-backed player
- High resolution audio: Supports up to 24bit/192Khz (max quality Qobuz offers)
- MPRIS support (control via [playerctl](https://github.com/altdesktop/playerctl) or other D-Bus client)
- Gapless playback
- Web UI 
- Terminal UI 

## Requirements

- [GStreamer v1.18+](https://gstreamer.freedesktop.org/documentation/installing/index.html) (comes with most/all current Linux)

## Installation

### Download Release

Download the tar.gz file for your supported OS from the releases page, extract the file and execute `qobuz-player` or copy it to the your `$PATH`.

### Build from source

On Debian, Arch and Fedora, `just build-player` should make a reasonable effort to install the necessary dependencies needed to build the app and then build it.

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

## TUI Controls

The TUI has full mouse support.

### Keyboard Shortcuts

| Command             | Key(s)                                 |
| ------------------- | -------------------------------------- |
| Now Playing         | <kbd>1</kbd>                           |
| My Playlists        | <kbd>2</kbd>                           |
| Search              | <kbd>3</kbd>                           |
| Enter URL           | <kbd>3</kbd>                           |
| Cycle elements      | <kbd>tab</kbd>                         |
| Play/Pause          | <kbd>space</kbd>                       |
| Next track          | <kbd>N</kbd>                           |
| Previous track      | <kbd>P</kbd>                           |
| Jump forward        | <kbd>l</kbd>                           |
| Jump backward       | <kbd>h</kbd>                           |
| Quit                | <kbd>ctrl</kbd> + <kbd>c</kbd>         |
| Move up in list     | <kbd>up arrow</kbd>                    |
| Move down in list   | <kbd>down arrow</kbd>                  |
| Select item in list | <kbd>enter</kbd>                       |
| Dismiss popup       | <kbd>esc</kbd>                         |

## Web UI

The player can start an embedded web interface. This is disabled by default and must be started with the `--web` argument. It also listens on `0.0.0.0:9888` by default,
but an inteface can be specified with the `--interface` argument.

Go to `http://<ip>:9888` to view the UI.

## Credits
Qobuz-player started as a fork of [hifi.rs](https://github.com/iamdb/hifi.rs) but has since diverged. 
Qobuz-player is mainly focused on the setup where a single boatd computer is connected to a speaker system.
