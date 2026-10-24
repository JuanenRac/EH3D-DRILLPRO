@echo off
rem Builds DRILLPRO (release, with the graphical front end).
cd /d "%~dp0"
where cargo >nul 2>nul
if errorlevel 1 (
    echo Rust is not installed. Get it from https://rustup.rs
    goto :end
)
cargo build --release --features gui
if errorlevel 1 (
    echo.
    echo Build FAILED.
) else (
    echo.
    echo Build OK: target\release\drillpro.exe
)
:end
if not defined NOPAUSE pause
