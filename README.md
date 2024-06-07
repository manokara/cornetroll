# cornetroll &#x1f950;

For those of you ricing up your custom tiling window manager desktop, I bring you cornetroll: the professional ricer's MPRIS controller! See, cornetroll is an anagram for controller and a cornet (*cornetto*) happens to be the name of a pastry, which is a kind of croissant, and those are quite twisty, almost like they're *rolling*. The layers!

## Features

- **Flexible formatting**: You can customize what and how the controller shows information.
- **Supports multiple players:** It keeps track of the available players and you can iterate through them.
- **Inline controls:** It can also emit polybar or EWW actions that send commands to the current player.
- **Scrollable metadata:** No, really - *scrollable metadata*. You know those huge-ass song titles that pollute your bar, tripping all over your other modules? Those problems are over, pal!


## Building

Make sure you have Rust 1.74.0+ and Cargo installed. You can build and install a release build at `~/.cargo/bin` by running:

```
cargo install --path .
```

## Usage

If called without arguments, cornetroll will start its main interface in "tail mode", meaning that unless terminated it will always constantly print lines of text with the current state every tick (300ms).

Every tick one character is scrolled in the `[metadata]` and `[info]` blocks (see [Display Format](#display-format) below) if their content are bigger than the allocated maximum character length. The scrolling is bidirectional, changing directions when reaching the start or end of the truncated content.

```
Usage: cornetroll [OPTIONS] [command]

Arguments:
  [command]  Which command to send to the current running instance [possible values: play, pause, stop, prev, next, prev-player, next-player, play-pause]

Options:
  -f, --display-format <display-format>
          How the player presents itself [default: "[prev] [play-pause] [next] [info] ┃ [metadata]"]
  -m, --metadata-format <metadata-format>
          What information about the song will be shown [default: "<[artist] - >[title]"]
  -r, --refresh-ticks <refresh-ticks>
          How many ticks to wait to refresh the player cache. [default: 10]
  -t, --markup-type <markup-type>
          What kind of markup should cornetroll output, if any. [default: polybar] [possible values: polybar, yuck, none]
  -e, --empty-msg <empty-msg>
          The text to show when no players are available [default: "\u{f057} no music playing"]
  -h, --help
          Print help
  -V, --version
          Print version
```

When running a release build, cornetroll creates a named pipe at `/tmp/cornetroll.$USER` and listens to it for any commands sent by `cornetroll [command]` (or written directly to the socket). As sockets go, you can't have more than one instance of cornetroll using it at the same time, so you'll get an error if the socket exists when trying to run cornetroll.

When running a debug build on the other hand, cornetroll turns into an interactive minimal TUI that allows you to control the player directly without using a socket for development purposes.

## Display Format

The formatting of what will be outputed by cornetroll every tick, defined by bracket-surrounded identifiers called *blocks* (e.g. `[foo]`). Blocks may have comma-separated arguments after a colon (e.g. `[foo:arg1,arg2]`). All arguments are optional, and you can skip the first one by leaving it empty (e.g. `[foo:,arg2]`).

### Action blocks

These blocks generate inline actions (when `--markup-type` is not `none`, see [Markup Types](#markup-types) below) that allow you to interact with cornetroll by issuing commands to itself.

- `[prev]`: Previous track button. This `previous` command to the current instance.
- `[play-pause]`: A dynamic play/pause button, changing according to the current playback status. Likewise, sending a `play-pause` command.
- `[next]`: Next track button. `next` command.

### Text blocks

- `[status]`: An action-less `play-pause`, just showing the current playback status. Note that the icons shown are the opposite of `play-pause`'s, plus the stop icon.
- `[info:show_total,show_name]`: Shows the current focused player in the following format: `current/total: name`. The two arguments control whether `total` and/or `name` will be shown, being either `true` or `false`. Both are true by default. `name` is on a 10-char scroll buffer, with the same wait ticks as metadata's default.
- `[metadata:buffer_size,wait_ticks]`: _This block is **mandatory**, if it's not present cornetroll will throw an error_. A scroll buffer showing the current player's song information. `buffer_size` is how many characters the scroll buffer will take (32 by default), and the metadata section will always be that many chars wide. When the metadata string is longer than buffer, the scroller waits `wait_ticks` ticks before it starts scrolling, and after every bounce.
- `[time:show_length,use_remaining]`: Show the current track's position in `MM:SS` format. Both arguments are bool. `show_length` will show the track's length alongside the position, as in `01:23/04:32`. If `use_remaining` is true, the length will show how much of the track is left instead. If `show_length` is false and `use_remaining` is true, only the remaining time will be shown.

### Icons used by blocks

Assuming you have FontAwesome's Regular and Solid styles installed and configured in your bar:

- `prev`:  (`\uf04a`).
- `next`:  (`\uf04e`).
- `play-pause`: ,  (play `\uf144`, pause `\uf28b`).
- `status` : , ,  (play `\uf144`, pause `\uf28b`, stop `\uf28d`).

## Markup Types

cornetroll can output its interface in three modes, chosen by the `--markup-type` command line option: `polybar`, `yuck` and `plain`. This mostly affects how actionable blocks generate inline clickable actions.

### `polybar`

If you're using polybar, cornetroll will emit polybar action markup for the action blocks, but will output the text blocks as they are. That means you can also use other kinds of polybar markup when setting the display format string.

### `yuck`

cornetroll will emit an EWW widget (`box`) with a `cornetroll` class, with each action block being a `button` and text blocks being `label`s with Pango markup enabled. Like with the `polybar` markup type, you can also define your own widgets in the display format string.

### `none`

Every display block will be output as plain text, without any markup of any kind. You can still write custom markup in the display format string when using polybar or EWW.

## Metadata Format

What will be shown inside the metadata block, using the same bracket syntax as the display format string.

- `[artist]`: The first/main artist
- `[artists]`: A list of artists separated by a comma.
- `[album]`: The song's album
- `[album-artist]`: The album's artist
- `[title]`: The song's title
- `[track]`: Track number

If the correspoding tag is not set, cornetroll will show it as `N/A`.

### Optional sections

You can make part of the metadata optional (e.g. only show artist name when the tag is actually set) by enclosing it in brackets `<...>` (e.g. `<[artist] - >[title]`). Essentially what this does is control the output of strings before and after a valid block. Some examples:

- `<[artist] ->[title]` - If `artist` is set, the result is `Artist - Title`, otherwise it's `Title`.
- `<[artist] - [title]>` - If only `artist` is set, the result is `Artist`. If only `title` is set, the result is ` - Title`. If both, `Artist - Title`.

Optionals can also be nested, allowing you to make somewhat complex metadata formats.
