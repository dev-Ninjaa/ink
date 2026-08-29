@echo off
setlocal
REM Launch MCP Inspector with INK_REPORT_DIR set so every tool call
REM writes timestamped JSON + a markdown summary into the reports folder.

set "INK_REPORT_DIR=E:\Codebase\Hackathon\ink-ibm\ink.mcpServer\reports"
if not exist "%INK_REPORT_DIR%" mkdir "%INK_REPORT_DIR%"

set "INK_MCP_EXE=E:\Codebase\Hackathon\ink-ibm\ink.mcpServer\target\debug\ink_mcp.exe"
if not exist "%INK_MCP_EXE%" (
    echo Building ink_mcp.exe ... 
    pushd E:\Codebase\Hackathon\ink-ibm\ink.mcpServer
    cargo build
    popd
)

echo INK_REPORT_DIR=%INK_REPORT_DIR%
echo.
echo In MCP Inspector, use:
echo   Command: %INK_MCP_EXE%
echo   Args:    (leave empty)
echo.
npx @modelcontextprotocol/inspector