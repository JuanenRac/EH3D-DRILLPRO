@echo off
rem Runs DRILLPRO. Builds it first if needed. Extra arguments are passed on:
rem   run.bat [FILE] [--lang en^|es^|it^|de^|fr^|pt^|zh^|ja] [--data DIR]
cd /d "%~dp0"
if not exist "target\release\drillpro.exe" (
    set NOPAUSE=1
    call "%~dp0build.bat"
    set NOPAUSE=
)
if not exist "target\release\drillpro.exe" (
    echo drillpro.exe was not built.
    goto :end
)
set DATA_ARGS=--data "%~dp0testdata"
for %%A in (%*) do if /i "%%A"=="--data" set DATA_ARGS=
"target\release\drillpro.exe" %DATA_ARGS% %*
:end
if not defined NOPAUSE pause
