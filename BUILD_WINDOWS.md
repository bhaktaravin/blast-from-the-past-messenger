# Building Blast From The Past on Windows

## Prerequisites

1. Install Rust: https://rustup.rs/
   - Download and run `rustup-init.exe`
   - Follow the installer prompts
   - Restart your terminal after installation

2. Install Git: https://git-scm.com/download/win
   - Download and install Git for Windows

## Build Steps

1. Clone the repository:
```powershell
git clone https://github.com/bhaktaravin/blast-from-the-past-messenger.git
cd blast-from-the-past-messenger
```

2. Build the release binary:
```powershell
cargo build --release --bin chatmessagediscordclone --features client
```

3. The executable will be at:
```
target\release\chatmessagediscordclone.exe
```

4. You can run it directly or copy it somewhere convenient:
```powershell
# Run directly
.\target\release\chatmessagediscordclone.exe

# Or copy to a folder
mkdir "C:\Program Files\Blast From The Past"
copy target\release\chatmessagediscordclone.exe "C:\Program Files\Blast From The Past\"
```

## Quick Run Script

Create a file called `run.bat` with:
```batch
@echo off
cargo run --release --bin chatmessagediscordclone --features client
```

Then just double-click `run.bat` to start the app!

## Troubleshooting

If you get errors about missing dependencies, you may need to install:
- Visual Studio Build Tools: https://visualstudio.microsoft.com/downloads/
  - Select "Desktop development with C++" workload

## Server URL

The app connects to: `wss://blast-from-the-past-messenger-production.up.railway.app`
