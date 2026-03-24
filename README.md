<p align="center">

<img width="150px" heigth="150px" src="ApertureLogo.png">

</p>

<h3 align="center">Aperture</h3>

<h4 align="center">Diagnostic tui for Windows power users</h4>

<br>

<p align="center"><a href="https://github.com/stylebending/Aperture/releases"><img src="https://img.shields.io/github/downloads/stylebending/Aperture/total?color=darkgreen&logo=github&label=GitHub%20Downloads&style=for-the-badge&labelColor=darkgreen"></a></p>

<br>

<h3 align="center">Quick Navigation</h3>

<p align="center"><a href="#quick-start-guide"><img src="https://img.shields.io/badge/🚀-Quick%20Start%20Guide-darkblue?style=for-the-badge&labelColor=darkblue"></a> <a href="#keybindings"><img src="https://img.shields.io/badge/🎹-Keybindings-darkblue?style=for-the-badge&labelColor=darkblue"></a></p>

## Installation (standalone/portable)

### Download and run the [latest release (1 MB executable)](https://github.com/stylebending/Aperture/releases/latest)
That's all! Run as admin to access all features.

### If you want to run the `aperture` command from any terminal:
**Just move Aperture.exe to a folder that is already in your user's PATH**  
Then restart your terminal and run `aperture` from anywhere!

## Installation (package managers)

### WinGet
`winget install aperture`

### Scoop
Scoop requires 100 GitHub stars or 2000 downloads to be in their Extras bucket, for now please use this installation command:  
`scoop install https://raw.githubusercontent.com/stylebending/scoop-bucket/refs/heads/main/bucket/Aperture.json`

### Chocolatey
`Coming very soon!`

Installing with these package managers automatically adds Aperture to your path. After running one of those installation commands, just close and re-open your terminal and you'll immediately be able to run `aperture` from any terminal.

## Usage

```bash
aperture
```

### Screenshots

**Locker Tab - Process Management**
```
┌────────────────────────────────────────┬───────────────────┐
│ Aperture [Locker] [Controller] [Nexus] │ Keys              │
├────────────────────────────────────────┼───────────────────┤
│ → Find and kill processes holding file │ Navigation        │
│   locks                                │ j/k     Move      │
│                                        │ ↑/↓     Move      │
│ ┌────────────────────────────────────┐ │ C-d/u   Page      │
│ │ Processes (Locker) [CPU▼] [45/230] │ │ Tab     SwitchTab │
│ │                                    │ │                   │
│ │ 1234  chrome.exe   15.2%   245.6MB │ │ Actions           │
│ │ 5678  firefox.exe   8.1%   189.2MB │ │ /       Search    │
│ │ 9012  notepad.exe   0.5%     4.2MB │ │ s/S     Sort      │
│ │ 3456  code.exe      3.2%    56.8MB │ │ f       FindLocks │
│ │ 7890  explorer.exe  2.1%    78.3MB │ │ K       Kill      │
│ │ ...                                │ │ r       Refresh   │
│ └────────────────────────────────────┘ │ Esc     ClearFilt │
│ Sort: CPU ▼                            │                   │
└────────────────────────────────────────┴───────────────────┘
```

**Controller Tab - Service Management**
```
┌────────────────────────────────────────┬───────────────────┐
│ ... [Controller] ...                   │ Keys              │
├────────────────────────────────────────┼───────────────────┤
│ → Start, stop, and manage Windows      │ Navigation        │
│   services                             │ j/k     Move      │
│                                        │ ...               │
│ ┌────────────────────────────────────┐ │                   │
│ │ Services (Controller) [Status▲]    │ │ Actions           │
│ │                                    │ │ /       Search    │
│ │ Windows Update        Running ...  │ │ s/S     Sort      │
│ │ Print Spooler         Running ...  │ │ Enter   Toggle    │
│ │ Bluetooth Service     Stopped ...  │ │ r       Refresh   │
│ │ ...                                │ │ Esc     ClearFilt │
│ └────────────────────────────────────┘ │                   │
│ Sort: Status ▲                         │                   │
└────────────────────────────────────────┴───────────────────┘
```

**File Lock Search Modal**
```
┌────────────────────────────────────────┐
│         Find Locking Processes         │
├────────────────────────────────────────┤
│ Path: C:\Users\Me\Documents\file.txt   │
│                                        │
│   Locking processes:                   │
│                                        │
│     PID:   5678  notepad.exe           │
│   ▶ PID:   9012  chrome.exe            │
│     PID:  12345  excel.exe             │
│                                        │
│ [/] Edit Path  [Enter] Search          │
│ [j/k] Navigate  [K] Kill  [Esc] Close  │
└────────────────────────────────────────┘
```

**Note:** Press `/` to enter input mode and type a file path. Enter a folder path to scan all files in that directory.

## Acknowledgements

### This project wouldn't be possible without:
- [**Rust**](https://rust-lang.org/) - Systems programming language making Windows API access safe and fast
- [**ratatui**](https://github.com/ratatui-org/ratatui) - The beautiful terminal user interface framework powering Aperture
- [**crossterm**](https://github.com/crossterm-rs/crossterm) - Cross-platform terminal manipulation and event handling
- [**tokio**](https://github.com/tokio-rs/tokio) - The asynchronous runtime enabling responsive UI updates
- [**windows-rs**](https://github.com/microsoft/windows-rs) - Microsoft's official Windows API bindings for Rust
- [**Vim**](https://www.vim.org/) / [**Neovim**](https://neovim.io/) - For the motion-based navigation philosophy

## Quick Start Guide

### Find What's Locking a File

Can't delete a file because it's "in use"? Aperture can find the culprit:

1. Press `f` to open the **File Lock Search** modal
2. Press `/` to enter input mode
3. Type the full path to the file (e.g., `C:\Users\You\file.txt`)
4. Press `Enter` to search
5. See which processes have the file locked
6. Navigate with `j`/`k` and press `K` to kill the process (requires admin)

**Tip:** Enter a folder path to scan all files in that directory and find all locks.

### Kill a Runaway Process

1. Switch to **Locker** tab (press `Tab` until you see "Locker")
2. Sort by CPU usage: Press `s` until title shows "CPU", then `S` to toggle direction
3. Find the process using high CPU
4. Press `K` to kill it (requires admin privileges)

### Manage Services

1. Switch to **Controller** tab
2. Sort by Status: Press `s` until title shows "Status"
3. Find the service you want to control
4. Press `Enter` to toggle start/stop (requires admin)

### View Process Tree

See hierarchical process relationships (parent/child):

1. Switch to **Locker** tab
2. Press `t` to toggle **Tree View**
3. Navigate the tree with `j`/`k`
4. Press `Space` to expand/collapse nodes (> / v indicators)
5. Search/filter still works in tree view - shows matching processes and their ancestor chain

### View Process Details

See detailed information about a process:

1. Switch to **Locker** tab
2. Navigate to a process with `j`/`k`
3. Press `d` to open **Process Details** modal
4. View loaded modules, parent PID, CPU, and memory usage
5. Press `K` in the modal to kill the process (requires admin)
6. Press `Esc` or `q` to close

### Export Data

Export all data to JSON or CSV format:

1. Press `e` to open the **Export** modal
2. Press `j` to export as JSON
3. Press `c` to export as CSV
4. Files are saved to your Documents folder with timestamps
5. Press `Esc` or `q` to cancel

**Export includes:** All processes, services, and network connections from all tabs

### Filter and Search

- Press `/` to enter search mode
- Type to filter the current list
- Press `Enter` to apply the filter and exit search mode
- Press `Esc` to clear the filter

**Example workflow:**
1. In Locker tab, press `/`
2. Type "chrome" - list filters to show only Chrome processes
3. Navigate with `j`/`k`
4. Press `Esc` to clear filter and see all processes again

### Navigate Large Lists

- `j`/`k` or `↑`/`↓` - Move one item at a time
- `Ctrl+D` - Page down (jump 10 items)
- `Ctrl+U` - Page up (jump 10 items)
- `gg` - Jump to first item
- `G` - Jump to last item
- `Tab`/`Shift+Tab` - Switch between tabs

### Sort Data

Each tab supports different sorting:

**Locker (Processes):**
- Press `s` to cycle: Name → PID → CPU → Memory
- Press `S` (Shift+s) to toggle ascending/descending
- Default: CPU descending (highest first)

**Controller (Services):**
- Press `s` to cycle: Name → Status → Type
- Press `S` to toggle order
- Default: Status ascending (Running first)

**Nexus (Connections):**
- Press `s` to cycle: State → PID → Protocol → Process
- Press `S` to toggle order
- Default: State ascending (ESTABLISHED first)

## Keybindings

| Category | Key | Action | Context | Description |
|----------|-----|--------|---------|-------------|
| **Navigation** | `Tab` / `Shift+Tab` | Switch tabs | Global | Move between Locker/Controller/Nexus |
| | `j` / `k` | Navigate | Lists | Move down/up one item |
| | `↑` / `↓` | Navigate | Lists | Alternative to j/k |
| | `Ctrl+D` | Page down | Lists | Jump down 10 items |
| | `Ctrl+U` | Page up | Lists | Jump up 10 items |
| | `gg` | Jump to first | Lists | Jump to first item |
| | `G` | Jump to last | Lists | Jump to last item |
| **Actions** | `/` | Toggle search | Global | Enter/exit search mode |
| | `Esc` | Clear/Cancel | Global | Clear filter, exit search, or close modal |
| | `s` | Cycle sort | Global | Change sort key (Name, PID, Status, etc.) |
| | `S` (Shift+s) | Toggle order | Global | Switch ascending/descending |
| | `r` | Refresh | Global | Force refresh current tab |
| | `f` | Find locks | Global | Open file lock search modal |
| | `e` | Export | Global | Open export format modal |
| **Locker** | `t` | Tree view | Locker only | Toggle hierarchical process tree view |
| | `Space` | Expand/Collapse | Locker only | Expand/collapse tree node (tree mode only) |
| | `d` | Details | Locker only | Show process details modal |
| | `K` | Kill process | Locker only | Kill selected process (admin) |
| **Controller** | `Enter` | Toggle service | Controller only | Start/stop selected service (admin) |
| **File Lock Modal** | `/` | Edit path | Modal | Enter input mode to type path |
| | `Enter` | Search | Modal | Execute search |
| | `j`/`k` | Navigate | Modal | Move up/down results |
| | `K` | Kill | Modal | Kill selected locking process |
| **System** | `q` | Quit | Global | Exit application |

### Search Mode Keybindings

When in search mode (`/`):
- Type characters to filter
- `Backspace` - Delete last character
- `Enter` - Apply filter and exit search
- `Esc` - Cancel search

### File Lock Search Modal

When file lock modal is open (`f`):
- Type file paths (one per line)
- `/` - Enter input mode to edit path (any key including j/k can now be typed)
- `Enter` - Search for locking processes
- `j`/`k` or `↑`/`↓` - Navigate results (normal mode only)
- `K` - Kill selected process (admin)
- `Esc` - Close modal (or cancel input mode)

**Directory Scanning:**
- Enter a folder path to scan all files in that directory
- Shows "Scanned X files - Found Y locks" with the count of files checked

### Export Modal

When export modal is open (`e`):
- `j` - Export to JSON format
- `c` - Export to CSV format
- `Esc` or `q` - Close modal without exporting

**Export Location:** Files are saved to your Documents folder with timestamps (e.g., `aperture_export_1234567890.json`)

### Process Details Modal

When process details modal is open (`d` in Locker tab):
- View process information: PID, name, parent PID, CPU%, memory
- View loaded modules (first 10, with count of additional modules)
- `K` - Kill the process (requires admin)
- `Esc` or `q` - Close modal

## Configuration

Aperture currently uses sensible defaults optimized for real-time performance:

| Setting | Default | Description |
|---------|---------|-------------|
| Data refresh interval | 2 seconds | How often to poll for new data |
| Navigation debounce | 50ms | Delay after navigation before accepting updates |
| CPU metrics interval | 1 second | How often to update CPU/memory usage |

**Note:** Configurable polling intervals are on the roadmap. Currently, these values are optimized for smooth real-time performance without overwhelming the system.

## Performance

Aperture is designed for real-time performance on Windows:

### Smart Update System
- **Change Detection**: Uses data hashing to only update when data actually changes
- **Navigation Debounce**: 50ms delay after navigation prevents cursor jumping during active use
- **Separate Concerns**: Filter operations apply instantly; only navigation triggers debounce
- **Cached Metrics**: CPU and memory values are cached to prevent flashing during temporary data unavailability

### Data Loading
- **Preload All Tabs**: Data for all tabs loads at startup, enabling instant tab switching
- **Background Updates**: All tabs refresh every 2 seconds in the background
- **Initial Load Bypass**: First data load happens immediately without debounce

### Why Not WMI?
Aperture uses direct Win32 APIs instead of WMI for maximum performance:
- WMI queries can take 500ms-2s per call
- Win32 APIs respond in <50ms
- Essential for smooth TUI experience with 2-second refresh rates

## Architecture

```
aperture/
├── src/
│   ├── main.rs          # Entry point, event loop, keybindings
│   ├── app.rs           # Application state, tab management
│   ├── ui/              # UI rendering
│   │   ├── mod.rs       # Layout, sidebar, status bar
│   │   ├── locker.rs    # Process tab UI with sorting
│   │   ├── controller.rs # Services tab UI with sorting
│   │   └── nexus.rs     # Network tab UI with sorting
│   ├── sys/             # Windows API abstractions
│   │   ├── process.rs   # Process enumeration, CPU/memory metrics
│   │   ├── service.rs   # SCM/Service control
│   │   ├── network.rs   # IP Helper/TCP-UDP connections
│   │   └── handle.rs    # File lock detection (Restart Manager)
│   └── state/           # Per-tab state with sorting
│       ├── locker.rs    # Process state, PID tracking
│       ├── controller.rs # Service state, name tracking
│       └── nexus.rs     # Connection state, key tracking
├── Cargo.toml
└── README.md
```

### Win32 APIs Used

| Feature | API |
|---------|-----|
| Process Enumeration | `EnumProcesses`, `QueryFullProcessImageNameW`, `CreateToolhelp32Snapshot`, `Process32FirstW` |
| Process Tree/Parent PID | `CreateToolhelp32Snapshot`, `Process32FirstW/NextW` |
| Process Metrics | `GetProcessTimes`, `GetProcessMemoryInfo` |
| Process Details | `EnumProcessModules`, `GetModuleBaseNameW`, `GetModuleFileNameExW` |
| Elevation Check | `OpenProcessToken`, `GetTokenInformation` |
| Service Management | `OpenSCManagerW`, `EnumServicesStatusExW`, `ControlService` |
| Network Connections (IPv4) | `GetExtendedTcpTable`, `GetExtendedUdpTable` |
| Network Connections (IPv6) | `GetExtendedTcpTable` (AF_INET6), `GetExtendedUdpTable` (AF_INET6) |
| File Lock Detection | `RmRegisterResources`, `RmGetList` (Restart Manager) |

## Roadmap

### Completed ✅
- [x] Process management with CPU/memory metrics
- [x] Handle search using Restart Manager API
- [x] Service management (start/stop)
- [x] Network connection monitoring (IPv4 + IPv6)
- [x] Smart sorting and filtering
- [x] Persistent sidebar with keybindings
- [x] Real-time navigation debounce (50ms)
- [x] Change detection and smart updates
- [x] Cached metrics to prevent flashing
- [x] **Process tree view** - Hierarchical parent/child relationships with expand/collapse
- [x] **Process details view** - Show loaded modules, parent PID, and process info
- [x] **Export to JSON/CSV** - Export all tab data with timestamps
- [x] **IPv6 support** - Full IPv6 TCP/UDP connection monitoring

### In Progress / TODO
- [ ] Real-time service status notifications via `NotifyServiceStatusChange`
  - Currently polls every 2s; would show instant service state changes
- [ ] Configurable polling intervals
  - Allow users to change 2s refresh rate
- [ ] Dark/light theme support
  - Currently uses terminal default colors

Aperture bridges the gap between the Linux `btop`/`lsof` experience and Windows' deep diagnostic capabilities (Processes, Services, and Network). Unlike cross-platform tools, Aperture focuses on Windows-specific pain points: file locks, service management, and process-to-socket mapping.

## Features

### The Locker (Process Management)
- View all running processes with PID, name, path, CPU%, and memory usage
- Real-time CPU and memory metrics with intelligent caching
- **Sort by**: Name, PID, CPU usage, Memory usage
- **Filter** processes by name, path, or PID
- **Kill processes** (requires admin - press `K`)
- **Find file locks** - Identify which processes are locking specific files (press `f`)
- **Process tree view** - Hierarchical parent/child relationships (press `t`)
- **Process details** - View loaded modules and detailed info (press `d`)

### The Controller (Service Management)
- List all Windows services with status, start type, and process ID
- **Start/Stop services** (requires admin - press `Enter`)
- **Sort by**: Name, Status, Service Type
- **Filter** services by name or display name

### The Nexus (Network Monitor)
- Real-time TCP/UDP connection listing (IPv4 and IPv6)
- Map connections to process PIDs and names
- View connection states (ESTABLISHED, LISTENING, etc.)
- **Sort by**: Connection State, PID, Protocol, Process Name
- **Filter** connections by address, port, PID, or process name

### UI Features
- **Vim Motions** keybindings for easy navigation
- **Permanent sidebar** with context-aware keybindings
- **Smart data caching** - All tabs preload for instant switching
- **50ms navigation debounce** - Smooth cursor movement without jitter
- **Change detection** - Only updates when data actually changes
- **Cached metrics** - CPU/memory values persist during temporary data unavailability
- **Export data** - Save all tab data to JSON or CSV (press `e`)

## License

MIT
