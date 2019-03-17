$cabAddress = "127.0.0.1"
$destinationAddress = "http://127.0.0.1"

$url = "https://api.github.com/repos/cscorley/kq-cab-relay/releases/latest"
$latest = Invoke-RestMethod -Method Get -Uri $url

$currentFile = "current"
$currentTag = ""
if (Test-Path $currentFile)
{
    $currentTag = Get-Content $currentFile
}

if ($currentTag -ne $latest.tag_name)
{

    $zipUrl = $latest.assets[0].browser_download_url
    $zipLocal = "kqcabrelay.zip"
    Invoke-WebRequest -Method Get -Uri $zipUrl -OutFile $zipLocal

    Set-Content -Path $currentFile -Value $latest.tag_name

    Expand-Archive $zipLocal -DestinationPath .
}

$errorCount = 0
$lastError = Get-Date
while ($true)
{
    try
    {
        .\kqcabrelay.exe --cab $cabAddress --destination $destinationAddress
    }
    catch
    {
        $errorCount += 1
        $currentTime = Get-Date
        if ($errorCount -gt 10 -and $lastError.AddMilliseconds(500) -gt $currentTime)
        {
            Write-Error -Message "Received too many errors recently, quitting"
            Write-Error -Message $_.Exception.Message
            exit
        }

        $lastError = $currentTime
    }

    Start-Sleep -Seconds 5
}
