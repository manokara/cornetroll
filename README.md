# cornetroll

For the brave ones using a WM-only desktop and polybar, and also happen to listen to a lot of music, I bring you the ultimate player-mpris-tail: cornetroll! See, it's an anagram for controller, "player-mpris-tail" is too big.

## Features

- **Flexible formatting**: You can customize what and how the controller shows information.
- **Supports multiple players:** It keeps track of the available players and you can iterate through them.
- **Inline controls:** It can also emit polybar actions that send commands to the current player. Though by default, there are no inline player switching controls as I prefer using the `scroll-*` actions for that.
- **Scrollable metadata:** No, really - *scrollable metadata*. You know those huge-ass song titles that pollute your bar, tripping over your other modules? Those problems are over, pal!


## Usage

### Options

- `-F, --format`: *Default: `[prev] [playpause] [next] [info] ┃ [metadata]`*.  See (Display Format](#display-format).
- `-m, --metadata-format`: *Default: `[artist] - [title]`*. See (Metadata Format)(#metadata-format).
- `-f, --framerate`: *Default: 300*. How many milliseconds the thread sleeps between each update. Make sure to adjust the module's `interval` in your polybar config accordingly.
- `-h, --hide-extended`: Remove extended information from the current player's *title*, such as `Artist - Song - Super Duper Edition - Feat Awesome Dood`. Specify it twice (`-hh`), and we will also remove parenthesized information such as `Artist - Song (Awesome Remix)`.

## Display Format

What will be shown in the controller at each update, each one of those bracket identifiers (`[foo]`) being called *blocks*. Blocks may have arguments, after a colon and separated by commas (`[foo:arg1,arg2]`). All arguments are optional, and you can skip the first one by leaving it empty (`[foo:,arg2]`).

- `[prev]`: Previous track button. This outputs a polybar action tag that sends a `previous` command to the current player through the controller's pipe.
- `[play-pause]`: A dynamic play/pause button, changing according to the current playback status. Likewise, sending a `play-pause` command.
- `[next]`: Next track button. `next` command.
- `[status]`: An action-less `play-pause`, just showing the current playback status. Note that the icons showed are the inverse of playpause's, plus the stop icon.
- `[prev-player]`: Button to switch to the previous player.
- `[next-player]`: Button to switch to the next player.
- `[info:show_total,show_name]`: Shows the current focused player in the following format: `current/total: name`. The two arguments control whether `total` and/or `name` will be shown, being either `true` or `false`. Both are true by default. `name` is on a 10-char scroll buffer, with the same wait ticks as metadata's default, but can also be changed with the `--info-size` option.
- `[metadata:buffer_size,scroll_timeout]`: _This block is **mandatory**, if it's not present cornetroll will throw an error_. A scroll buffer showing the current player's song information. `buffer_size` is how many characteres the scroll buffer will take (32 by default), and the metadata section will always be that many chars wide. When the metadata string is longer than buffer, the scroller waits `scroll_timeout` ticks before it starts scrolling, and after every bounce.
- `[time:show_length,use_remaining]`: Show the current track's position in `HH:MM` format. Both arguments are bool. `show_length` will show the track's length alongside the position, as in `01:23/04:32`. If `use_remaining` is true, the length will show how much of the track is left instead. If `show_length` is false and `use_remaining` is true, only the remaining time will be shown.

### Icons used by the inline controls

Assuming you have FontAwesome's Regular and Solid styles intalled and configured in polybar:

- `prev`:  (`\uff34`).
- `next`:  (`\uf04e`).
- `play-pause`: ,  (`\uf144`, `\uf28b`).
- `status` : , ,  (`\uf144`, `\uf28b`, `\uf28d`).
- `prev-player`: (`\uffff`).
- `next-player`: (`\uffff`).

## Metadata Format

What track information will be shown inside the metadata block.

- `[artist]`: The first/main artist
- `[artists]`: A list of artists separated by a comma.
- `[album]`: The song's album
- `[album-artist]`: The album's artist
- `[title]`: The song's title
- `[track]`: Track number
