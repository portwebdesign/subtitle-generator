@echo off
chcp 65001 >nul
title Altyazı Üretici - Kurulum

echo ════════════════════════════════════════════════════════
echo    MP4 Altyazı Üretici - Bileşen Kurulumu
echo ════════════════════════════════════════════════════════
echo.

set "HEDEF=%~dp0"
set "BIN=%HEDEF%bin"
set "MODELLER=%HEDEF%models"

:: Klasörleri oluştur
if not exist "%BIN%" mkdir "%BIN%"
if not exist "%MODELLER%" mkdir "%MODELLER%"

echo [1/4] Klasörler hazırlandı.

:: FFmpeg kontrolü
if exist "%BIN%\ffmpeg.exe" (
    echo [2/4] ffmpeg.exe zaten mevcut, atlanıyor.
) else (
    echo [2/4] FFmpeg indiriliyor...
    echo       Bu birkaç dakika sürebilir ^(~80MB^)...
    
    :: PowerShell ile indir
    powershell -Command "& {
        $url = 'https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip'
        $zip = '%TEMP%\ffmpeg.zip'
        Write-Host '  Indiriliyor...'
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        (New-Object System.Net.WebClient).DownloadFile($url, $zip)
        Write-Host '  Aciliyor...'
        Add-Type -Assembly System.IO.Compression.FileSystem
        $z = [System.IO.Compression.ZipFile]::OpenRead($zip)
        $entry = $z.Entries | Where-Object { $_.Name -eq 'ffmpeg.exe' }
        if ($entry) {
            [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, '%BIN%\ffmpeg.exe', $true)
            Write-Host '  ffmpeg.exe kopyalandi!'
        }
        $z.Dispose()
        Remove-Item $zip -Force
    }"
    
    if exist "%BIN%\ffmpeg.exe" (
        echo       ffmpeg.exe basariyla indirildi!
    ) else (
        echo       UYARI: FFmpeg indirilemedi!
        echo       Manuel olarak ffmpeg.exe dosyasini "%BIN%" klasörüne kopyalayin.
        echo       İndirme adresi: https://ffmpeg.org/download.html
    )
)

:: whisper.cpp kontrolü  
if exist "%BIN%\whisper-cli.exe" (
    echo [3/4] whisper-cli.exe zaten mevcut, atlanıyor.
) else (
    echo [3/4] whisper.cpp indiriliyor...
    echo       Bu birkaç dakika sürebilir...
    
    powershell -Command "& {
        $url = 'https://github.com/ggerganov/whisper.cpp/releases/download/v1.7.5/whisper-blas-bin-x64.zip'
        $zip = '%TEMP%\whisper.zip'
        Write-Host '  Indiriliyor...'
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        try {
            (New-Object System.Net.WebClient).DownloadFile($url, $zip)
            Write-Host '  Aciliyor...'
            Add-Type -Assembly System.IO.Compression.FileSystem
            $z = [System.IO.Compression.ZipFile]::OpenRead($zip)
            foreach ($entry in $z.Entries) {
                if ($entry.Name -match 'whisper|main' -and $entry.Name -like '*.exe') {
                    $dest = '%BIN%\whisper-cli.exe'
                    [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $dest, $true)
                    Write-Host ('  ' + $entry.Name + ' -> whisper-cli.exe kopyalandi!')
                    break
                }
            }
            $z.Dispose()
            Remove-Item $zip -Force
        } catch {
            Write-Host ('  HATA: ' + $_.Exception.Message)
        }
    }"
    
    if exist "%BIN%\whisper-cli.exe" (
        echo       whisper-cli.exe basariyla indirildi!
    ) else (
        echo       UYARI: whisper.cpp indirilemedi!
        echo       Manuel indirme: https://github.com/ggerganov/whisper.cpp/releases
        echo       whisper-cli.exe dosyasini "%BIN%" klasörüne kopyalayin.
    )
)

:: Model indir
echo [4/4] Whisper tiny model indiriliyor (75MB)...
if exist "%MODELLER%\ggml-tiny.bin" (
    echo       ggml-tiny.bin zaten mevcut, atlanıyor.
) else (
    powershell -Command "& {
        $url = 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin'
        $dest = '%MODELLER%\ggml-tiny.bin'
        Write-Host '  Tiny model indiriliyor (75MB)...'
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        try {
            $wc = New-Object System.Net.WebClient
            $wc.DownloadFile($url, $dest)
            Write-Host '  ggml-tiny.bin indirildi!'
        } catch {
            Write-Host ('  HATA: ' + $_.Exception.Message)
            Write-Host '  Manuel indirme: https://huggingface.co/ggerganov/whisper.cpp'
        }
    }"
    
    if exist "%MODELLER%\ggml-tiny.bin" (
        echo       Model basariyla indirildi!
    ) else (
        echo       UYARI: Model indirilemedi!
    )
)

echo.
echo ════════════════════════════════════════════════════════
echo    Kurulum Tamamlandı - Durum:
echo ════════════════════════════════════════════════════════

if exist "%BIN%\ffmpeg.exe"       (echo    [OK] ffmpeg.exe) else (echo    [EKSIK] ffmpeg.exe)
if exist "%BIN%\whisper-cli.exe"  (echo    [OK] whisper-cli.exe) else (echo    [EKSIK] whisper-cli.exe)
if exist "%MODELLER%\ggml-tiny.bin" (echo    [OK] ggml-tiny.bin modeli) else (echo    [EKSIK] ggml-tiny.bin modeli)

echo.
echo Uygulamayı başlatmak için: altyazi_uretici.exe
echo.
pause
