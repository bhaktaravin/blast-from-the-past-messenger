@echo off
REM Test launch script for the application
echo Testing application launch...
echo.

REM Build release
echo Building release...
cargo build --release
if %ERRORLEVEL% neq 0 (
    echo FAILED: Build did not complete successfully
    exit /b 1
)

echo.
echo Build successful! Release executable at: target\release\chatmessagediscordclone.exe
echo.
echo You can now run the installer or launch the app directly.
pause
