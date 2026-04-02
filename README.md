# `terminal-snapshot`

This is a small program to run a program under a terminal emulator and snapshot what is visible on the terminal after it exits.

## Usage 
```
Usage: terminal-snapshot [OPTIONS] <COMMAND>...

Arguments:
  <COMMAND>...  The command to run.

Options:
  -c, --cols <COLS>      Number of columns in the terminal emulator [default: 80]
  -r, --rows <ROWS>      Number of rows in the terminal emulator [default: 24]
  -o, --output <OUTPUT>  Output path. Use `-` for stdout [default: -]
  -h, --help             Print help
```

## Example

```sh
terminal-snapshot --rows 18 --cols 100 --timeout 1 -- btm
┌ CPU ─ 1.73 1.55 1.33 ─────────────────────────────────────────────────────────────┐┌─────────────┐
│100%│                                                                              ││CPU    Use   │
│    │                                                                              ││All          │
│    │                                                                              ││AVG    4%    │
│  0%│                                                                              ││CPU0   0%    │
└───────────────────────────────────────────────────────────────────────────────────┘└─────────────┘
┌ Memory ────────────────────────────────────────────────┐┌ Temperatures ──────────────────────────┐
│100%│                                                   ││Sensor(s)▲                      Temp(t) │
│    │                                                   ││DEADBEEF-0000-0000-00A0-000000… 42°C    │
│  0%│                                                   │└────────────────────────────────────────┘
│    └───────────────────────────────────────────────────│┌ Disks ─────────────────────────────────┐
│  60s                                                 0s││/dev/sda1 /          xxxGB     xGB      │
└────────────────────────────────────────────────────────┘└────────────────────────────────────────┘
┌ Network ───────────────────────────────────────┐┌ Processes ─────────────────────────────────────┐
│128.6│                                          ││PID(p)    Name(n)           CPU%(c)▼  Mem%(m)   │
│ 85.8│                                          ││1335890   btm               1.7%      0.1%      │
│ 42.9│                                          ││11322     Xorg              0.8%      0.4%      │
└────────────────────────────────────────────────┘└────────────────────────────────────────────────┘
````