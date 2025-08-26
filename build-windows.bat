@echo off
echo Building PolyCue for Windows...
echo.

cargo build --release

if %ERRORLEVEL% EQU 0 (
    echo.
    echo Build successful!
    echo Executable created: target\release\polycue.exe
    echo.
    echo You can now run: target\release\polycue.exe
) else (
    echo.
    echo Build failed! Please check the error messages above.
    pause
    exit /b %ERRORLEVEL%
)

pause
