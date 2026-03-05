@echo off
setlocal

REM 参数说明：
REM 1: Base URL（默认 http://localhost:48760）
REM 2: ApiKey（默认 test-key）
REM 3: Model（默认 gpt-5.3-codex）
REM 4: 模式，可选 stream（流式）或留空（非流式）

set "BASE=%~1"
if "%BASE%"=="" set "BASE=http://localhost:48760"

set "API_KEY=%~2"
if "%API_KEY%"=="" set "API_KEY=test-key"

set "MODEL=%~3"
if "%MODEL%"=="" set "MODEL=gpt-5.3-codex"

set "STREAM_ARG="
if /I "%~4"=="stream" set "STREAM_ARG=-Stream"

pwsh -NoProfile -ExecutionPolicy Bypass -File "%~dp0chat_tools_hit_probe.ps1" -Base "%BASE%" -ApiKey "%API_KEY%" -Model "%MODEL%" %STREAM_ARG%
set "EXIT_CODE=%ERRORLEVEL%"

exit /b %EXIT_CODE%

