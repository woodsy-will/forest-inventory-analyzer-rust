@echo off
cd /d "%~dp0"
echo Starting Forest Inventory Analyzer...
echo.
echo Open your browser to: http://localhost:8080
echo Press Ctrl+C to stop the server.
echo.
start http://localhost:8080
forest-analyzer.exe serve
