@echo off
setlocal

REM Option A: set DATABASE_URL here (uncomment and fill in)
REM set DATABASE_URL=postgres://user:pass@host:5432/retrochat


if exist .env (
  for /f "usebackq tokens=1,* delims==" %%A in (".env") do (
    if /i "%%A"=="DATABASE_URL" set "DATABASE_URL=%%B"
  )
)

if "%DATABASE_URL%"=="" (
  echo DATABASE_URL is not set.
  echo Add it to .env or set it in your shell.
  echo Example: postgres://postgres:[YOUR-PASSWORD]@wcllqcbmnnxkllkmdkid.db.us-west-2.nhost.run:5432/wcllqcbmnnxkllkmdkid
  pause
  exit /b 1
)

start "AOL Server" server.exe
REM Give the server a moment to bind before launching the client.
timeout /t 1 >nul
start "AOL Client" chatmessagediscordclone.exe

endlocal
