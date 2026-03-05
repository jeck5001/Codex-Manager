param(
    [string]$Base = "http://localhost:48760",
    [string]$ApiKey = "test-key",
    [string]$Model = "gpt-5.3-codex",
    [string]$Endpoint = "/v1/chat/completions",
    [int]$TimeoutSeconds = 90,
    [switch]$Stream
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# 中文注释：使用超长工具名验证“请求侧缩短 + 响应侧还原”是否生效。
$LongToolName =
    "mcp__tool_server_namespace_for_codex_manager_gateway_adapter_alignment__very_long_tool_operation_name"

function New-ChatBodyJson
{
    param(
        [string]$BodyModel,
        [string]$ToolName,
        [bool]$EnableStream
    )

    $body = @{
        model = $BodyModel
        stream = $EnableStream
        messages = @(
            @{
                role = "user"
                content = "请调用指定工具，参数 path=README.md，并返回工具调用。"
            }
        )
        tools = @(
            @{
                type = "function"
                "function" = @{
                    name = $ToolName
                    description = "Read file by path"
                    parameters = @{
                        type = "object"
                        properties = @{
                            path = @{
                                type = "string"
                                description = "file path"
                            }
                        }
                        required = @("path")
                    }
                }
            }
        )
        tool_choice = @{
            type = "function"
            "function" = @{
                name = $ToolName
            }
        }
    }
    return ($body | ConvertTo-Json -Depth 100 -Compress)
}

function Print-NonStreamResult
{
    param(
        $ResponseObject,
        [string]$ExpectedToolName
    )

    $choice0 = $null
    if ($ResponseObject.choices -and $ResponseObject.choices.Count -gt 0)
    {
        $choice0 = $ResponseObject.choices[0]
    }

    $toolCalls = @()
    if ($choice0 -and $choice0.message -and $choice0.message.tool_calls)
    {
        $toolCalls = @($choice0.message.tool_calls)
    }
    $toolHit = ($toolCalls.Count -gt 0)
    $toolName = ""
    $toolArguments = ""
    if ($toolHit)
    {
        $toolName = [string]$toolCalls[0].'function'.name
        $toolArguments = [string]$toolCalls[0].'function'.arguments
    }

    $finishReason = ""
    if ($choice0)
    {
        $finishReason = [string]$choice0.finish_reason
    }
    $restoredOk = $toolHit -and ($toolName -eq $ExpectedToolName)

    Write-Host "=== Chat Tools Hit Probe (non-stream) ==="
    Write-Host "tool_hit             : $toolHit"
    Write-Host "finish_reason        : $finishReason"
    Write-Host "tool_name_returned   : $toolName"
    Write-Host "tool_name_len        : $($toolName.Length)"
    Write-Host "expected_tool_name   : $ExpectedToolName"
    Write-Host "restored_name_ok     : $restoredOk"
    Write-Host "tool_arguments       : $toolArguments"
    Write-Host ""
    Write-Host "raw_response_json:"
    Write-Output ($ResponseObject | ConvertTo-Json -Depth 100)
}

function Print-StreamResult
{
    param(
        [string[]]$Lines,
        [string]$ExpectedToolName
    )

    $toolHit = $false
    $toolName = ""
    $finishReason = ""
    $usageSeen = $false

    foreach ($line in $Lines)
    {
        if (-not $line.StartsWith("data:"))
        {
            continue
        }
        $payload = $line.Substring(5).Trim()
        if ($payload -eq "[DONE]" -or [string]::IsNullOrWhiteSpace($payload))
        {
            continue
        }
        $obj = $null
        try
        {
            $obj = $payload | ConvertFrom-Json -Depth 100
        } catch
        {
            continue
        }

        if ($obj.usage)
        {
            $usageSeen = $true
        }

        if ($obj.choices -and $obj.choices.Count -gt 0)
        {
            $choice0 = $obj.choices[0]
            if ($choice0.finish_reason)
            {
                $finishReason = [string]$choice0.finish_reason
            }
            if ($choice0.delta -and $choice0.delta.tool_calls -and $choice0.delta.tool_calls.Count -gt 0)
            {
                $toolHit = $true
                $name = [string]$choice0.delta.tool_calls[0].'function'.name
                if (-not [string]::IsNullOrWhiteSpace($name))
                {
                    $toolName = $name
                }
            }
        }
    }

    $restoredOk = $toolHit -and ($toolName -eq $ExpectedToolName)
    Write-Host "=== Chat Tools Hit Probe (stream) ==="
    Write-Host "tool_hit             : $toolHit"
    Write-Host "finish_reason        : $finishReason"
    Write-Host "tool_name_returned   : $toolName"
    Write-Host "tool_name_len        : $($toolName.Length)"
    Write-Host "expected_tool_name   : $ExpectedToolName"
    Write-Host "restored_name_ok     : $restoredOk"
    Write-Host "usage_seen           : $usageSeen"
    Write-Host ""
    Write-Host "raw_sse_lines:"
    $Lines | ForEach-Object { Write-Output $_ }
}

function Invoke-ChatProbe
{
    param(
        [string]$BaseUrl,
        [string]$UrlPath,
        [string]$BearerKey,
        [string]$BodyJson,
        [int]$MaxSeconds,
        [bool]$EnableStream,
        [string]$ExpectedToolName
    )

    $url = ($BaseUrl.TrimEnd("/") + $UrlPath)
    if (-not $EnableStream)
    {
        $headers = @{
            Authorization = "Bearer $BearerKey"
        }
        $response = Invoke-RestMethod -Method Post -Uri $url -Headers $headers -Body $BodyJson -ContentType "application/json" -TimeoutSec $MaxSeconds
        Print-NonStreamResult -ResponseObject $response -ExpectedToolName $ExpectedToolName
        return
    }

    # 中文注释：流式场景使用 curl 读取 SSE 原始帧，便于观察 delta.tool_calls。
    if (-not (Get-Command curl.exe -ErrorAction SilentlyContinue))
    {
        throw "curl.exe not found"
    }
    $tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("chat_tools_probe_" + [Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
    $bodyFile = Join-Path $tempDir "body.json"
    $outFile = Join-Path $tempDir "stream.txt"
    try
    {
        [System.IO.File]::WriteAllText($bodyFile, $BodyJson, [System.Text.Encoding]::UTF8)
        $args = @(
            "-sS", "-N",
            "-o", $outFile,
            "-X", "POST", $url,
            "-H", "Authorization: Bearer $BearerKey",
            "-H", "Content-Type: application/json",
            "--data-binary", "@$bodyFile",
            "--max-time", "$MaxSeconds"
        )
        & curl.exe @args
        if ($LASTEXITCODE -ne 0)
        {
            throw "curl failed with exit code $LASTEXITCODE"
        }
        $lines = @()
        if (Test-Path $outFile)
        {
            $lines = Get-Content -Path $outFile -Encoding UTF8
        }
        Print-StreamResult -Lines $lines -ExpectedToolName $ExpectedToolName
    } finally
    {
        if (Test-Path $tempDir)
        {
            Remove-Item -Recurse -Force $tempDir
        }
    }
}

try
{
    $json = New-ChatBodyJson -BodyModel $Model -ToolName $LongToolName -EnableStream $Stream.IsPresent
    Invoke-ChatProbe -BaseUrl $Base -UrlPath $Endpoint -BearerKey $ApiKey -BodyJson $json -MaxSeconds $TimeoutSeconds -EnableStream $Stream.IsPresent -ExpectedToolName $LongToolName
} catch
{
    Write-Error ("chat tools probe failed: " + $_.Exception.Message)
    exit 1
}
