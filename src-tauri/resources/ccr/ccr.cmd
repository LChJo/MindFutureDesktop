@echo off
setlocal

:: CCR wrapper script for Moonton Claude Desktop terminal
:: This allows users to run "ccr" commands in the built-in terminal

set "SCRIPT_DIR=%~dp0"
set "CLI_PATH=%SCRIPT_DIR%cli.cjs"

:: Try to find Node.js in the following order:
:: 1. MoontonClaudeDesktop's bundled Node.js in ~/.MoontonClaudeDesktop
:: 2. Bundled Node.js in resources folder
:: 3. System PATH

if exist "%USERPROFILE%\.MoontonClaudeDesktop\nodejs\node.exe" (
    set "NODE_PATH=%USERPROFILE%\.MoontonClaudeDesktop\nodejs\node.exe"
    goto :run
)

if exist "%SCRIPT_DIR%..\nodejs-windows\node.exe" (
    set "NODE_PATH=%SCRIPT_DIR%..\nodejs-windows\node.exe"
    goto :run
)

:: Fall back to system node
where node >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    set "NODE_PATH=node"
    goto :run
)

echo Error: Node.js not found. Please ensure Node.js is installed.
exit /b 1

:run
"%NODE_PATH%" "%CLI_PATH%" %*
