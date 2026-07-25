# Fix llm.rs - remove chunk index from prompt and context
$path = "D:\Projects\vedo-help\backend\src\shared\llm.rs"
$content = [System.IO.File]::ReadAllText($path)

# Replacement 1: SYSTEM_PROMPT
$old1 = "Always cite the source document name and chunk when referencing specific information."
$new1 = "Always cite the source document name when referencing specific information. Do NOT mention chunk numbers or chunk indices."
$content = $content.Replace($old1, $new1)

# Replacement 2: Context format string (with surrounding quotes)
$old2 = '"[Source: {} (chunk {})]\n{}"'
$new2 = '"[Source: {}]\n{}"'
$content = $content.Replace($old2, $new2)

# Replacement 3: Remove c.index from format args
$old3 = "c.document_name, c.index, c.text"
$new3 = "c.document_name, c.text"
$content = $content.Replace($old3, $new3)

[System.IO.File]::WriteAllText($path, $content, [System.Text.UTF8Encoding]::new($false))
Write-Host "OK - applied 3 replacements"
