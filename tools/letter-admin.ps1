[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Backend,
    [string]$TargetLabel = 'LOCAL'
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

function Invoke-LetterBackend {
    param([string[]]$Arguments)
    $lines = & $Backend @Arguments 2>&1
    $exitCode = $LASTEXITCODE
    $jsonLine = @($lines | ForEach-Object { $_.ToString() } | Where-Object { $_.Trim().StartsWith('{') })[-1]
    if (-not $jsonLine) { throw "Yardımcı program geçerli cevap vermedi: $($lines -join [Environment]::NewLine)" }
    $result = $jsonLine | ConvertFrom-Json
    if ($exitCode -ne 0 -or -not $result.ok) { throw $result.error }
    return $result
}

function New-Label([string]$text, [int]$x, [int]$y, [int]$width = 120) {
    $c = New-Object System.Windows.Forms.Label
    $c.Text = $text; $c.Location = New-Object Drawing.Point($x, $y)
    $c.Size = New-Object Drawing.Size($width, 22)
    return $c
}

function New-TextBox([int]$x, [int]$y, [int]$width, [string]$text = '') {
    $c = New-Object System.Windows.Forms.TextBox
    $c.Location = New-Object Drawing.Point($x, $y); $c.Size = New-Object Drawing.Size($width, 24)
    $c.Text = $text
    return $c
}

function New-Button([string]$text, [int]$x, [int]$y, [int]$width = 100) {
    $c = New-Object System.Windows.Forms.Button
    $c.Text = $text; $c.Location = New-Object Drawing.Point($x, $y)
    $c.Size = New-Object Drawing.Size($width, 29)
    return $c
}

function Set-GridData($grid, $rows) {
    $table = New-Object System.Data.DataTable
    if ($rows -and @($rows).Count -gt 0) {
        foreach ($property in @($rows)[0].PSObject.Properties) { [void]$table.Columns.Add($property.Name) }
        foreach ($row in @($rows)) {
            $dataRow = $table.NewRow()
            foreach ($property in $row.PSObject.Properties) { $dataRow[$property.Name] = [string]$property.Value }
            [void]$table.Rows.Add($dataRow)
        }
    }
    $grid.DataSource = $table
    $grid.AutoResizeColumns([System.Windows.Forms.DataGridViewAutoSizeColumnsMode]::DisplayedCells)
}

$form = New-Object System.Windows.Forms.Form
$form.Text = "JSTKO Letter Item Admin — $TargetLabel"
$form.StartPosition = 'CenterScreen'; $form.Size = New-Object Drawing.Size(1120, 790)
$form.MinimumSize = New-Object Drawing.Size(1000, 700)
$form.Font = New-Object Drawing.Font('Segoe UI', 9)

$tabs = New-Object System.Windows.Forms.TabControl
$tabs.Dock = 'Fill'
$sendTab = New-Object System.Windows.Forms.TabPage; $sendTab.Text = 'Item Gönder'
$historyTab = New-Object System.Windows.Forms.TabPage; $historyTab.Text = 'Son Gönderiler'
[void]$tabs.TabPages.Add($sendTab); [void]$tabs.TabPages.Add($historyTab); $form.Controls.Add($tabs)

$recipientSearch = New-TextBox 125 18 260
$recipientButton = New-Button 'Karakter Ara' 395 15 110
$characterGrid = New-Object System.Windows.Forms.DataGridView
$characterGrid.Location = New-Object Drawing.Point(15, 50); $characterGrid.Size = New-Object Drawing.Size(520, 205)
$characterGrid.ReadOnly = $true; $characterGrid.AllowUserToAddRows = $false; $characterGrid.SelectionMode = 'FullRowSelect'
$characterGrid.MultiSelect = $false

$itemSearch = New-TextBox 660 18 260
$itemButton = New-Button 'Item Ara' 930 15 100
$itemGrid = New-Object System.Windows.Forms.DataGridView
$itemGrid.Location = New-Object Drawing.Point(550, 50); $itemGrid.Size = New-Object Drawing.Size(520, 205)
$itemGrid.ReadOnly = $true; $itemGrid.AllowUserToAddRows = $false; $itemGrid.SelectionMode = 'FullRowSelect'
$itemGrid.MultiSelect = $false

$recipient = New-TextBox 140 282 230
$itemId = New-TextBox 500 282 160
$itemName = New-TextBox 790 282 280; $itemName.ReadOnly = $true
$count = New-Object System.Windows.Forms.NumericUpDown
$count.Location = New-Object Drawing.Point(140, 325); $count.Size = New-Object Drawing.Size(100, 24); $count.Minimum = 1; $count.Maximum = 9999; $count.Value = 1
$durability = New-Object System.Windows.Forms.NumericUpDown
$durability.Location = New-Object Drawing.Point(500, 325); $durability.Size = New-Object Drawing.Size(100, 24); $durability.Minimum = 0; $durability.Maximum = 32767
$expiry = New-Object System.Windows.Forms.NumericUpDown
$expiry.Location = New-Object Drawing.Point(790, 325); $expiry.Size = New-Object Drawing.Size(100, 24); $expiry.Minimum = 0; $expiry.Maximum = 3650
$sender = New-TextBox 140 368 230 'JSTKO Admin'
$subject = New-TextBox 500 368 570 'Test item delivery'
$message = New-TextBox 140 411 930 'This item was sent for gameplay testing.'
$sendButton = New-Button 'Letter ile Gönder' 140 458 170
$clearButton = New-Button 'Temizle' 320 458 100
$status = New-Object System.Windows.Forms.Label
$status.Location = New-Object Drawing.Point(140, 505); $status.Size = New-Object Drawing.Size(930, 90)
$status.ForeColor = [Drawing.Color]::DarkSlateGray

$sendTab.Controls.AddRange(@(
    (New-Label 'Karakter ara:' 15 21), $recipientSearch, $recipientButton, $characterGrid,
    (New-Label 'Item ID / adı:' 550 21), $itemSearch, $itemButton, $itemGrid,
    (New-Label 'Alıcı karakter:' 15 285), $recipient,
    (New-Label 'Item ID:' 390 285 100), $itemId,
    (New-Label 'Item adı:' 680 285 100), $itemName,
    (New-Label 'Adet:' 15 328), $count,
    (New-Label 'Dayanıklılık:' 390 328 105), $durability,
    (New-Label 'Süre (gün, 0=süresiz):' 620 328 165), $expiry,
    (New-Label 'Gönderen:' 15 371), $sender,
    (New-Label 'Konu:' 390 371 100), $subject,
    (New-Label 'Mesaj:' 15 414), $message, $sendButton, $clearButton,
    (New-Label 'Durum:' 15 508), $status
))

$historyRefresh = New-Button 'Yenile' 15 15 100
$historyGrid = New-Object System.Windows.Forms.DataGridView
$historyGrid.Location = New-Object Drawing.Point(15, 55); $historyGrid.Size = New-Object Drawing.Size(1055, 620)
$historyGrid.Anchor = 'Top,Bottom,Left,Right'; $historyGrid.ReadOnly = $true
$historyGrid.AllowUserToAddRows = $false; $historyGrid.SelectionMode = 'FullRowSelect'
$historyTab.Controls.AddRange(@($historyRefresh, $historyGrid))

$recipientButton.Add_Click({
    try { Set-GridData $characterGrid (Invoke-LetterBackend @('characters', $recipientSearch.Text)).data }
    catch { [System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'Hata', 'OK', 'Error') | Out-Null }
})
$recipientSearch.Add_KeyDown({ if ($_.KeyCode -eq 'Enter') { $recipientButton.PerformClick() } })
$characterGrid.Add_CellDoubleClick({
    if ($_.RowIndex -ge 0) { $recipient.Text = [string]$characterGrid.Rows[$_.RowIndex].Cells['name'].Value }
})

$itemButton.Add_Click({
    try { Set-GridData $itemGrid (Invoke-LetterBackend @('items', $itemSearch.Text)).data }
    catch { [System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'Hata', 'OK', 'Error') | Out-Null }
})
$itemSearch.Add_KeyDown({ if ($_.KeyCode -eq 'Enter') { $itemButton.PerformClick() } })
$itemGrid.Add_CellDoubleClick({
    if ($_.RowIndex -ge 0) {
        $row = $itemGrid.Rows[$_.RowIndex]
        $itemId.Text = [string]$row.Cells['id'].Value; $itemName.Text = [string]$row.Cells['name'].Value
        $durability.Value = [Math]::Max(0, [int]$row.Cells['duration'].Value)
        if ([int]$row.Cells['countable'].Value -eq 0) { $count.Value = 1; $count.Enabled = $false } else { $count.Enabled = $true }
    }
})

$clearButton.Add_Click({
    $recipient.Clear(); $itemId.Clear(); $itemName.Clear(); $count.Value = 1
    $count.Enabled = $true; $durability.Value = 0; $expiry.Value = 0; $status.Text = ''
})
$sendButton.Add_Click({
    try {
        $sendButton.Enabled = $false; $status.ForeColor = [Drawing.Color]::DarkSlateGray; $status.Text = 'Gönderiliyor...'
        $result = Invoke-LetterBackend @('send', $recipient.Text, $itemId.Text, [string]$count.Value,
            [string]$durability.Value, [string]$expiry.Value, $subject.Text, $message.Text, $sender.Text)
        $status.ForeColor = [Drawing.Color]::DarkGreen
        $status.Text = "Başarılı. Letter ID: $($result.letter_id) | $($result.recipient) | $($result.item_id) - $($result.item_name) x$($result.count)`r`nOyuncu Letter penceresini yenileyip eki teslim alabilir."
    } catch {
        $status.ForeColor = [Drawing.Color]::DarkRed; $status.Text = $_.Exception.Message
        [System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'Gönderim başarısız', 'OK', 'Error') | Out-Null
    } finally { $sendButton.Enabled = $true }
})

$historyRefresh.Add_Click({
    try { Set-GridData $historyGrid (Invoke-LetterBackend @('recent')).data }
    catch { [System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'Hata', 'OK', 'Error') | Out-Null }
})
$tabs.Add_SelectedIndexChanged({ if ($tabs.SelectedTab -eq $historyTab) { $historyRefresh.PerformClick() } })
$form.Add_Shown({
    try {
        $ping = Invoke-LetterBackend @('ping')
        $status.Text = "PostgreSQL bağlantısı hazır. $($ping.characters) karakter bulundu. Listeden seçim yapmak için satıra çift tıklayın."
        $recipientButton.PerformClick()
    } catch {
        $status.ForeColor = [Drawing.Color]::DarkRed; $status.Text = $_.Exception.Message
    }
})

[void]$form.ShowDialog()
