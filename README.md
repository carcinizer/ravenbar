# Ravenbar - more than a status bar

Ravenbar is a customizable status bar for Linux and X11 written in Rust. The key feature differentiating it from eg. Polybar is its interactivity. User can change almost every aspect of a bar on certain events. This makes it possible to easily create interactive widgets, autohiding bars, HUDs that disappear on mouse hover and possibly more.

It's currently WIP, so things may change.



## How to write a config:

#### 1. The general structure

A very simple config may look something like this:

```
{
    "height": 25,
    "screenwidth": 0.7
    "alignment": "N",

    "defaults": {
        "background": "#333333"
        "foreground": "#EEEEFF"
    }

    "widgets_left": [
        {
            "command": "date +%H:%M"
            "command.on_hover": "date +%H:%M:%S"
        }
    ]
}
```

This config will create a bar on top of a screen that takes up 70% of the horizontal space, with a clock on the left that can be hovered on to reveal seconds.

As you can see, the **properties** such as `height` or `background` may be followed by a dot and an **event** to change the widget when the condition is met.



#### 2. Events

Note: Every mouse event is only activated for widget under the cursor in case of widget properties. Some events have optional

| Event name                   | Description                                                                                                                                          |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `on_hover`                   | Activates when the mouse is hovering over a bar (in case of bar properties) or a widget (in case of widget properties)                               |
| `on_press[.button]`          | Activates when the mouse is pressed once. The optional `button` parameter may be "left", "middle", "right", "scroll_up", "scroll_down" or a number.  |
| `on_press_cont[.button]`     | Activates when the mouse is beign pressed.                                                                                                           |
| `on_release[.button]`        | Activates when the mouse is released once.                                                                                                           |
| `on_release_cont[.button]`   | Activates when the mouse is beign released.                                                                                                          |
| `on_file_changed.{filename}` | Activates when the file modification date is changed. `filename` is relative to config directory and does no character escaping beyond JSON standard |



#### 3. Non-property fields

These can not be affected by events.



Bar fields:

| Field name        | Description                                                                                                                                                 |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `defaults`        | Describes default widget properties for every widget.                                                                                                       |
| `template.{name}` | Creates a template which widgets can inherit properties from.                                                                                               |
| `widgets_left`    | Widgets on the left side of a bar.                                                                                                                          |
| `widgets_right`   | Widgets on the right side of a bar.                                                                                                                         |
| `font[.{name}]`   | A string (font name) or a list of strings (default font and its fallbacks, may be useful for eg. emoji). Widgets can refer to a non-default font by `name`. |

Widget fields:

| Field name           | Description                                                               |
| -------------------- | ------------------------------------------------------------------------- |
| `template[.{event}]` | A pseudoproperty that allows widget to inherit certain template's widgets |



#### 4. Bar properties

| Property      | Description                                                | Default |
| ------------- | ---------------------------------------------------------- | ------- |
| `alignment`   | Bar's place on screen, may be one of N, NE, NW, S, SE, SW  | N       |
| `height`      | Bar's height in pixels                                     | 24      |
| `screenwidth` | Bar's width relative to screen width                       | 1.0     |
| `xoff`        | Bar's x offset in pixels                                   | 0       |
| `yoff`        | Bar's y offset in pixels                                   | 0       |
| `solid`       | If true, maximized windows won't cover/be covered by a bar | true    |
| `above`       | Describes whether bar is displayed above other windows     | false   |
| `below`       | Describes whether bar is displayed below other windows     | false   |
| `visible`     | Describes whether bar is visible                           | true    |



#### 5. Widget properties

| Property         | Description                                                                                                                                                                                               | Default |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| `foreground`     | Widget's foreground (text). See Appearance section for more detail.                                                                                                                                       | #FFFFFF |
| `background`     | Widget's background. See Appearance section for more detail.                                                                                                                                              | #222233 |
| `warn`           | Threshold above which the text will turn yellow (float). The bar will read output searching for a number (also including k/Ki/etc. suffixes) and determine whether the reading is above a given treshold. | +inf    |
| `critical`       | Threshold above which the text will turn red (float).                                                                                                                                                     | +inf    |
| `dim`            | Threshold above which the text will turn bright_black (float).                                                                                                                                            | -inf    |
| `font`           | The font name.                                                                                                                                                                                            | default |
| `command`        | Command to execute and display output of. See Commands section for more detail.                                                                                                                           |         |
| `action`         | Command to execute without displaying output. See Commands section for more detail.                                                                                                                       |         |
| `border_factor`  | How thick/thin the vertical widget borders are. 1.0 means the text is the same height as bar, 0.5 means half the height of a bar etc.                                                                     | 0.75    |
| `interval`       | How often to repeat command, in seconds (may be float).                                                                                                                                                   | 5.0     |
| `black`          | Colors, as used by the terminal, described in the same way as background/foreground.                                                                                                                      | #000000 |
| `red`            | See `black`                                                                                                                                                                                               | #AA0000 |
| `green`          | See `black`                                                                                                                                                                                               | #00AA00 |
| `yellow`         | See `black`                                                                                                                                                                                               | #AAAA00 |
| `blue`           | See `black`                                                                                                                                                                                               | #0000AA |
| `magenta`        | See `black`                                                                                                                                                                                               | #AA00AA |
| `cyan`           | See `black`                                                                                                                                                                                               | #00AAAA |
| `white`          | See `black`                                                                                                                                                                                               | #AAAAAA |
| `bright_black`   | See `black`                                                                                                                                                                                               | #777777 |
| `bright_red`     | See `black`                                                                                                                                                                                               | #FF0000 |
| `bright_green`   | See `black`                                                                                                                                                                                               | #00FF00 |
| `bright_yellow`  | See `black`                                                                                                                                                                                               | #FFFF00 |
| `bright_blue`    | See `black`                                                                                                                                                                                               | #0000FF |
| `bright_magenta` | See `black`                                                                                                                                                                                               | #FF00FF |
| `bright_cyan`    | See `black`                                                                                                                                                                                               | #00FFFF |
| `bright_white`   | See `black`                                                                                                                                                                                               | #FFFFFF |



#### 6. Appearance

Currently appearance can be a string with one or more hex colors (#RRGGBB or #RRGGBBAA) separated by semicolons, for example "#232334;#23233400" describes a vertical gradient fading from dark grey to transparency.



#### 7. Commands

Command may be one of the following things:

- `"#{text}"` - Display text.
- `"{command}"` - Execute command periodically and display its output.
- `"|{command}"` - Pipe command - Run a command in the background and display the last line written to output.
- `[{cmd1}, {cmd2}, ...]` - Combine outputs of several commands
- `{"type": "{type}", ...}` - Builtin command, 
  
  

A type may be one of the following:

| Type                    | Description                                                                                                                                                                                                                                     | Options                                                                                                                 |
|:----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `cpu_usage`             | CPU usage                                                                                                                                                                                                                                       | core - optional - integer                                                                                               |
| `cpu_freq`              | CPU frequency                                                                                                                                                                                                                                   | `core` - optional - integer                                                                                             |
| `(mem\|swap\|disk)_{A}` | Describes RAM/Swap/Disk statistics:  A is one of usage, percent, total, free - total usage/usage percentage/total capacity/free space, for example `mem_free` - amount of RAM available                                                         | `mountpoint` - required - disks only - mountpoint of a disk                                                             |
| `net_{A}_{B}[_{C}]`     | Net statistics: A is "upload" or "download", B is one of: "bits", "bytes", "packets", "errors", C may be nothing (per second), "since" (since last update) or "total". Example - `net_download_bytes` - current download speed in (k/M/G)bits/s | `network` - network interface name, as reported by `ip addr`                                                            |
| `alsa_volume_get`       | Get ALSA volume                                                                                                                                                                                                                                 | `card` - optional - card name                                                                                           |
| `alsa_volume_set`       | Set ALSA volume                                                                                                                                                                                                                                 | `card` - optional - card name, `volume` - volume change, for example "+5%", "-3%" or "5%" (change volume to exactly 5%) |



TODO:

- Multi-monitor support (may be worked around with an offset)
- Icon/Image support
- Use inotify to monitor files
- DBus support
- More builtin widgets
