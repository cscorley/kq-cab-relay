$cabAddress = "127.0.0.1"
$destinationAddress = "http://127.0.0.1"
$scoreDestinationAddress = "http://127.0.0.1:8888/api/a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11"

$url = "https://api.github.com/repos/cscorley/kq-cab-relay/releases/latest"

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
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

    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
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
        .\kqcabrelay.exe --cab $cabAddress --destination $destinationAddress --score-destination $scoreDestinationAddress
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
