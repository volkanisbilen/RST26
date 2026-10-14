@echo off
cd /d "%~dp0"
python hgpus.py pull
python hgpus.py decode
pause
