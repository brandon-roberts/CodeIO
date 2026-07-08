# CodeIO on Android (Termux)

Run CodeIO directly on your phone — no cloud, no cost.

## First time
1. Install **Termux** from F-Droid (not the Play Store — that version is outdated).
2. Open Termux and run these two lines:

```
pkg install -y wget
wget -O setup.sh https://raw.githubusercontent.com/brandon-roberts/CodeIO/main/tools/termux/setup.sh && bash setup.sh
```

That installs Rust/git/protobuf, clones CodeIO, builds it, and installs a `cio` launcher.
The first build is slow (several minutes) — that's normal.

## Everyday use
```
cio menu        # friendly numbered menu — start here
cio doctor      # check your environment
cio run ~/CodeIO/examples/tables.cio
cio repl        # interactive prompt
```

## Making it feel like an app
- **Termux:Widget** (from F-Droid) puts a launcher on your home screen. Create
  `~/.shortcuts/CodeIO` containing `cio menu` and it becomes a one-tap home-screen icon.
- **Termux:Styling** gives you nicer fonts/colors.
- Later: `codeio serve` will add a browser UI you open at `localhost` — coming soon.
